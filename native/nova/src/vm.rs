use crate::bytecode::{Chunk, Func, Instr, Program};
use crate::messages;
use crate::value::{arith, nova_eq, num_cmp, ArithError, Value};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

pub struct VmError {
    pub msg: String,
}

fn err(msg: String) -> VmError {
    VmError { msg }
}

pub fn truth(v: &Value) -> Result<bool, VmError> {
    match v {
        Value::Bool(b) => Ok(*b),
        _ => Err(err(messages::interp::condition_not_bool())),
    }
}

enum Iter {
    List(std::vec::IntoIter<Value>),
    Text { s: String, i: usize },
    Range(i64, i64, i64),
}

impl Iter {
    fn next_value(&mut self) -> Option<Value> {
        match self {
            Iter::List(it) => it.next(),
            Iter::Text { s, i } => {
                let bytes = s.as_bytes();
                if *i >= bytes.len() {
                    return None;
                }
                let ch = s[*i..].chars().next()?;
                *i += ch.len_utf8();
                Some(Value::Text(ch.to_string()))
            }
            Iter::Range(cur, end, step) => {
                if (*step > 0 && cur <= end) || (*step < 0 && cur >= end) {
                    let v = *cur;
                    *cur += *step;
                    Some(Value::Int(v.into()))
                } else {
                    None
                }
            }
        }
    }
}

struct Frame {
    func_idx: usize,
    ip: usize,
    locals: Option<HashMap<String, Value>>,
    iters: Vec<Iter>,
    handlers: Vec<(u16, usize, usize, bool)>,
}

#[derive(Default)]
pub struct Vm {
    globals: HashMap<String, Value>,
    out: String,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    history: HashMap<String, Vec<Value>>,
    redo: HashMap<String, Vec<Value>>,
}

impl Vm {
    pub fn new() -> Self {
        Vm::default()
    }

    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.out)
    }

    pub fn run_program(&mut self, prog: &Program) -> Result<(), VmError> {
        self.frames.push(Frame { func_idx: 0, ip: 0, locals: None, iters: vec![], handlers: vec![] });
        self.exec(prog)
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, VmError> {
        let func = Func {
            name: "<expr>".into(),
            params: vec![],
            names: vec![],
            chunk: chunk.clone(),
        };
        let prog = Program { funcs: vec![func] };
        self.run_program(&prog)?;
        Ok(self.stack.pop().unwrap_or(Value::Nothing))
    }

    fn step(&mut self, prog: &Program, instr: Instr) -> Result<(), VmError> {
        let fi = self.frames.last().ok_or_else(|| err("no frame".into()))?.func_idx;
        match instr {
                Instr::Halt => return Ok(()),
                Instr::Ret => {
                    let v = self.stack.pop().unwrap_or(Value::Nothing);
                    self.frames.pop();
                    if self.frames.is_empty() {
                        return Ok(());
                    }
                    self.stack.push(v);
                }
                Instr::Call(fidx, argc) => {
                    let at = self.stack.len() - argc as usize;
                    let args: Vec<Value> = self.stack.split_off(at);
                    self.enter_func(prog, fidx as usize, args)?;
                }
                Instr::CallName(nidx, argc) => {
                    let at = self.stack.len() - argc as usize;
                    let args: Vec<Value> = self.stack.split_off(at);
                    let name = prog.funcs[fi].names[nidx as usize].clone();
                    let fidx = prog
                        .funcs
                        .iter()
                        .position(|f| f.name == name)
                        .ok_or_else(|| err(messages::interp::func_not_found(&name)))?;
                    self.enter_func(prog, fidx, args)?;
                }
                Instr::Print(count, newline) => {
                    let at = self.stack.len() - count as usize;
                    let parts: Vec<String> = self.stack.split_off(at).iter().map(render).collect();
                    self.out.push_str(&parts.join(""));
                    if newline {
                        self.out.push('\n');
                    }
                }
                Instr::LoadName(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let v = self.load_name(fi, &name)?;
                    self.stack.push(v);
                }
                Instr::StoreName(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let v = self.stack.pop().unwrap_or(Value::Nothing);
                    if self.history.contains_key(&name) {
                        self.history.get_mut(&name).unwrap().push(v.clone());
                        self.redo.remove(&name);
                    }
                    self.store_name(fi, name, v);
                }
                Instr::Track(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let cur = self.load_name_for_track(fi, &name);
                    self.history.entry(name).or_insert_with(|| vec![cur]);
                }
                Instr::Undo(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let usable = self
                        .history
                        .get(&name)
                        .map(|h| h.len() >= 2)
                        .unwrap_or(false);
                    if !usable {
                        return Err(err(messages::interp::no_changes(&name, "undo")));
                    }
                    let h = self.history.get_mut(&name).unwrap();
                    let cur = h.pop().unwrap();
                    let restored = h.last().unwrap().clone();
                    self.redo.entry(name.clone()).or_default().push(cur);
                    self.store_name(fi, name, restored);
                }
                Instr::Redo(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let next = match self.redo.get_mut(&name) {
                        Some(r) if !r.is_empty() => r.pop().unwrap(),
                        _ => return Err(err(messages::interp::no_changes(&name, "redo"))),
                    };
                    if let Some(h) = self.history.get_mut(&name) {
                        h.push(next.clone());
                    }
                    self.store_name(fi, name, next);
                }
                Instr::AddToName(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let delta = self.stack.pop().unwrap_or(Value::Nothing);
                    let cur = self.load_name(fi, &name)?;
                    match &cur {
                        Value::List(items) => {
                            items.borrow_mut().push(delta);
                        }
                        Value::Int(_) | Value::Float(_) => {
                            let newv = arith("plus", &cur, &delta).map_err(|e| {
                                err(arith_msg("plus", &cur, &delta, e))
                            })?;
                            self.store_existing(fi, &name, newv)?;
                        }
                        other => {
                            return Err(err(messages::interp::add_needs_list_or_num(
                                other.type_name(),
                            )));
                        }
                    }
                }
                Instr::IterNew => {
                    let v = self.stack.pop().unwrap_or(Value::Nothing);
                    let it = match v {
                        Value::List(items) => Iter::List(items.borrow().clone().into_iter()),
                        Value::Text(s) => Iter::Text { s, i: 0 },
                        _ => return Err(err(messages::interp::each_needs_seq(""))),
                    };
                    self.frames.last_mut().unwrap().iters.push(it);
                }
                Instr::IterTimesNew => {
                    let v = self.stack.pop().unwrap_or(Value::Nothing);
                    let n = as_i64(&v)
                        .ok_or_else(|| err(messages::interp::times_needs_num(v.type_name())))?;
                    let n = n.max(0);
                    self.frames.last_mut().unwrap().iters.push(Iter::Range(1, n, 1));
                }
                Instr::IterCountNew => {
                    let b = self.stack.pop().unwrap_or(Value::Nothing);
                    let a = self.stack.pop().unwrap_or(Value::Nothing);
                    let ai = as_i64(&a)
                        .ok_or_else(|| err(messages::interp::counting_needs_num(a.type_name())))?;
                    let bi = as_i64(&b)
                        .ok_or_else(|| err(messages::interp::counting_needs_num(b.type_name())))?;
                    let step = if ai <= bi { 1 } else { -1 };
                    self.frames.last_mut().unwrap().iters.push(Iter::Range(ai, bi, step));
                }
                Instr::IterNext(end) => {
                    let next = self.frames.last_mut().unwrap().iters.last_mut().unwrap().next_value();
                    match next {
                        Some(v) => self.stack.push(v),
                        None => {
                            self.frames.last_mut().unwrap().iters.pop();
                            self.frames.last_mut().unwrap().ip = end as usize;
                        }
                    }
                }
                Instr::IterClose => {
                    self.frames.last_mut().unwrap().iters.pop();
                }
                Instr::Jump(t) => self.frames.last_mut().unwrap().ip = t as usize,
                Instr::JumpIfFalsePop(t) => {
                    let v = self.stack.pop().unwrap_or(Value::Nothing);
                    if !truth(&v)? {
                        self.frames.last_mut().unwrap().ip = t as usize;
                    }
                }
                Instr::JumpIfTruePop(t) => {
                    let v = self.stack.pop().unwrap_or(Value::Nothing);
                    if truth(&v)? {
                        self.frames.last_mut().unwrap().ip = t as usize;
                    }
                }
                Instr::JumpIfFalse(t) => {
                    let v = self.stack.last().ok_or_else(|| err("stack underflow".into()))?;
                    if !truth(v)? {
                        self.frames.last_mut().unwrap().ip = t as usize;
                    }
                }
                Instr::JumpIfTrue(t) => {
                    let v = self.stack.last().ok_or_else(|| err("stack underflow".into()))?;
                    if truth(v)? {
                        self.frames.last_mut().unwrap().ip = t as usize;
                    }
                }
                Instr::MustBeBool => {
                    let t = self.stack.last().ok_or_else(|| err("stack underflow".into()))?;
                    truth(t)?;
                }
                Instr::MakeList(n) => {
                    let at = self.stack.len() - n as usize;
                    let items = self.stack.split_off(at);
                    self.stack.push(Value::List(Rc::new(RefCell::new(items))));
                }
                Instr::Pop => {
                    self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                }
                Instr::Const(c) => {
                    let v = prog.funcs[fi].chunk.consts[c as usize].clone();
                    self.stack.push(v);
                }
                Instr::Add => self.bin_arith("plus")?,
                Instr::Sub => self.bin_arith("minus")?,
                Instr::Mul => self.bin_arith("times")?,
                Instr::Div => self.bin_arith("divided")?,
                Instr::Mod => self.bin_arith("mod")?,
                Instr::Eq | Instr::Ne => {
                    let (b, a) = self.pop2()?;
                    let eq = nova_eq(&a, &b);
                    self.stack.push(Value::Bool(if matches!(instr, Instr::Eq) { eq } else { !eq }));
                }
                Instr::Lt | Instr::Lte | Instr::Gt | Instr::Gte => {
                    let (b, a) = self.pop2()?;
                    let ord = ordering(&a, &b)?;
                    let res = match instr {
                        Instr::Lt => ord == Ordering::Less,
                        Instr::Lte => ord != Ordering::Greater,
                        Instr::Gt => ord == Ordering::Greater,
                        _ => ord != Ordering::Less,
                    };
                    self.stack.push(Value::Bool(res));
                }
                Instr::Contains => {
                    let (needle, hay) = self.pop2()?;
                    let res = contains(&hay, &needle)?;
                    self.stack.push(Value::Bool(res));
                }
                Instr::StartsWith | Instr::EndsWith => {
                    let (affix, s) = self.pop2()?;
                    let res = match (&s, &affix) {
                        (Value::Text(s), Value::Text(p)) => {
                            if matches!(instr, Instr::StartsWith) {
                                s.starts_with(p.as_str())
                            } else {
                                s.ends_with(p.as_str())
                            }
                        }
                        _ => return Err(err(messages::interp::contains_needs_str_or_list())),
                    };
                    self.stack.push(Value::Bool(res));
                }
                Instr::IsNumber => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    self.stack.push(Value::Bool(v.is_number()));
                }
                Instr::IsEmpty => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let e = match &v {
                        Value::Text(t) => t.is_empty(),
                        Value::List(l) => l.borrow().is_empty(),
                        _ => false,
                    };
                    self.stack.push(Value::Bool(e));
                }
                Instr::MakeThing { cls, fields } => {
                    let cls_name = prog.funcs[fi].names[cls as usize].clone();
                    let n = fields.len();
                    let at = self.stack.len() - n;
                    let values: Vec<Value> = self.stack.split_off(at);
                    let mut map = HashMap::new();
                    for (fidx, val) in fields.iter().zip(values) {
                        let fname = prog.funcs[fi].names[*fidx as usize].clone();
                        map.insert(fname, val);
                    }
                    let thing = crate::value::Thing { cls: cls_name, fields: map };
                    self.stack.push(Value::Thing(Rc::new(RefCell::new(thing))));
                }
                Instr::GetField(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let obj = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    match &obj {
                        Value::Thing(t) => {
                            let v = t.borrow().fields.get(&name).cloned();
                            match v {
                                Some(v) => self.stack.push(v),
                                None => {
                                    let (cls, mut names) = {
                                        let t = t.borrow();
                                        (t.cls.clone(), t.fields.keys().cloned().collect::<Vec<_>>())
                                    };
                                    names.sort();
                                    return Err(err(messages::interp::thing_missing_field(
                                        &cls,
                                        &name,
                                        &names.join(", "),
                                    )));
                                }
                            }
                        }
                        Value::Nothing => {
                            return Err(err(messages::interp::field_of_nothing(&name)));
                        }
                        other => {
                            return Err(err(messages::interp::cannot_read_field(
                                &name,
                                &render(other),
                            )));
                        }
                    }
                }
                Instr::StoreField(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    let value = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let obj = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    match &obj {
                        Value::Thing(t) => {
                            t.borrow_mut().fields.insert(name, value);
                        }
                        _ => {
                            return Err(err(messages::interp::field_needs_thing()));
                        }
                    }
                }
                Instr::CopyOf => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    self.stack.push(v.deep_copy());
                }
                Instr::Dup => {
                    let v = self.stack.last().ok_or_else(|| err("stack underflow".into()))?.clone();
                    self.stack.push(v);
                }
                Instr::UnknownThing(i) => {
                    let name = prog.funcs[fi].names[i as usize].clone();
                    return Err(err(messages::interp::unknown_thing(&name)));
                }
                Instr::Not => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let b = truth(&v)?;
                    self.stack.push(Value::Bool(!b));
                }
                Instr::TryPush(catch_ip, push_err) => {
                    let iters_len = self.frames.last().unwrap().iters.len();
                    self.frames
                        .last_mut()
                        .unwrap()
                        .handlers
                        .push((catch_ip, self.stack.len(), iters_len, push_err));
                }
                Instr::TryPop => {
                    self.frames.last_mut().unwrap().handlers.pop();
                }
                Instr::RequireCheck => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    if !truth(&v)? {
                        return Err(err(messages::interp::contract_failed("requires")));
                    }
                }
                Instr::EnsureCheck => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    if !truth(&v)? {
                        let fname = prog.funcs[self.frames.last().unwrap().func_idx].name.clone();
                        return Err(err(messages::interp::ensure_failed(&fname)));
                    }
                }
            }
        Ok(())
    }

    fn exec(&mut self, prog: &Program) -> Result<(), VmError> {
        loop {
            let Some(frame) = self.frames.last() else {
                return Ok(());
            };
            let (fi, ip) = (frame.func_idx, frame.ip);
            let code_len = prog.funcs[fi].chunk.code.len();
            if ip >= code_len {
                return Ok(());
            }
            let instr = prog.funcs[fi].chunk.code[ip].clone();
            self.frames.last_mut().unwrap().ip += 1;
            if let Err(e) = self.step(prog, instr) {
                let msg = e.msg;
                let mut handled = false;
                loop {
                    let handler = self.frames.last().and_then(|f| f.handlers.last().copied());
                    match handler {
                        Some((catch_ip, stack_len, iters_len, push_err)) => {
                            self.frames.last_mut().unwrap().handlers.pop();
                            self.stack.truncate(stack_len);
                            let fr = self.frames.last_mut().unwrap();
                            fr.iters.truncate(iters_len);
                            fr.ip = catch_ip as usize;
                            if push_err {
                                self.stack.push(Value::Text(msg.clone()));
                            }
                            handled = true;
                        }
                        None => {
                            if self.frames.pop().is_none() {
                                break;
                            }
                        }
                    }
                    if handled {
                        break;
                    }
                }
                if !handled {
                    return Err(VmError { msg });
                }
            }
        }
    }

    fn load_name_for_track(&mut self, fi: usize, name: &str) -> Value {
        self.load_name(fi, name).unwrap_or(Value::Nothing)
    }

    fn is_main(&self, fi: usize) -> bool {
        fi == 0
    }

    fn load_name(&self, fi: usize, name: &str) -> Result<Value, VmError> {
        if let Some(locals) = &self.frames.last().unwrap().locals {
            if let Some(v) = locals.get(name) {
                return Ok(v.clone());
            }
        }
        if !self.is_main(fi) {
            if let Some(v) = self.globals.get(name) {
                return Ok(v.clone());
            }
        } else if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
        }
        Err(err(messages::interp::var_not_found(name)))
    }

    fn store_name(&mut self, fi: usize, name: String, v: Value) {
        if let Some(locals) = &mut self.frames.last_mut().unwrap().locals {
            if locals.contains_key(&name) {
                locals.insert(name.clone(), v);
                return;
            }
        }
        if self.globals.contains_key(&name) || self.is_main(fi) {
            #[allow(clippy::map_entry)]
            self.globals.insert(name, v);
            return;
        }
        self.frames
            .last_mut()
            .unwrap()
            .locals
            .get_or_insert_with(HashMap::new)
            .insert(name, v);
    }

    #[allow(clippy::needless_pass_by_value)]
    fn store_existing(&mut self, _fi: usize, name: &str, v: Value) -> Result<(), VmError> {
        if let Some(locals) = &mut self.frames.last_mut().unwrap().locals {
            if locals.contains_key(name) {
                locals.insert(name.to_string(), v);
                return Ok(());
            }
        }
        if self.globals.contains_key(name) {
            self.globals.insert(name.to_string(), v);
            return Ok(());
        }
        Err(err(messages::interp::var_not_found(name)))
    }

    fn enter_func(&mut self, prog: &Program, fidx: usize, args: Vec<Value>) -> Result<(), VmError> {
        let params = &prog.funcs[fidx].params;
        if params.len() != args.len() {
            let name = prog.funcs[fidx].name.clone();
            let hint = format!("{name} with {}", params.join(" and "));
            return Err(err(messages::interp::func_arity(&name, params.len(), args.len(), &hint)));
        }
        let mut locals = HashMap::new();
        for (p, v) in params.iter().cloned().zip(args) {
            locals.insert(p, v);
        }
        self.frames.push(Frame { func_idx: fidx, ip: 0, locals: Some(locals), iters: vec![], handlers: vec![] });
        Ok(())
    }

    fn pop2(&mut self) -> Result<(Value, Value), VmError> {
        let b = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
        let a = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
        Ok((b, a))
    }

    fn bin_arith(&mut self, op: &'static str) -> Result<(), VmError> {
        let (b, a) = self.pop2()?;
        match arith(op, &a, &b) {
            Ok(v) => {
                self.stack.push(v);
                Ok(())
            }
            Err(e) => Err(err(arith_msg(op, &a, &b, e))),
        }
    }
}

fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => i.to_string().parse().ok(),
        _ => None,
    }
}

pub fn render(v: &Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => crate::lexer::fmt_float(*f),
        Value::Text(t) => t.clone(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Nothing => "nothing".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.borrow().iter().map(render).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Thing(t) => format!("{}(...)", t.borrow().cls),
    }
}

fn contains(hay: &Value, needle: &Value) -> Result<bool, VmError> {
    match hay {
        Value::Text(s) => match needle {
            Value::Text(n) => Ok(s.contains(n.as_str())),
            _ => Err(err(messages::interp::contains_needs_str_or_list())),
        },
        Value::List(items) => Ok(items.borrow().iter().any(|v| nova_eq(v, needle))),
        _ => Err(err(messages::interp::contains_needs_str_or_list())),
    }
}

fn ordering(a: &Value, b: &Value) -> Result<Ordering, VmError> {
    num_cmp(a, b).ok_or_else(|| err(messages::interp::ordering_needs_numbers(a.type_name())))
}

fn arith_msg(_op: &str, _a: &Value, _b: &Value, e: ArithError) -> String {
    match e {
        ArithError::TypeMismatch { left, right } => {
            messages::interp::plus_type_mismatch(left, right)
        }
        ArithError::DivByZero => messages::interp::div_by_zero(),
        ArithError::ModByZero => messages::interp::mod_by_zero(),
        ArithError::OnNothing => messages::interp::arith_on_nothing(),
    }
}






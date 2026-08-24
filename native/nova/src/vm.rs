use crate::ast::SKind;
use crate::bytecode::{Chunk, Env, Func, Instr, Program};
use crate::messages;
use crate::value::{arith, nova_eq, num_cmp, ArithError, LoadedModule, ModuleVal, NativeModule, Value};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct VmError {
    pub msg: String,
    pub signal: bool,
}

fn err(msg: String) -> VmError {
    VmError { msg, signal: false }
}

fn signal_err(msg: String) -> VmError {
    VmError { msg, signal: true }
}

const STDLIBS: [&str; 8] = ["file", "json", "list", "math", "random", "test", "text", "time"];

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
    prog: Rc<Program>,
    func_idx: usize,
    ip: usize,
    locals: Option<HashMap<String, Value>>,
    iters: Vec<Iter>,
    handlers: Vec<(u16, usize, usize, u8)>,
}

#[derive(Default)]
pub struct Vm {
    out: String,
    stack: Vec<Value>,
    frames: Vec<Frame>,
    history: HashMap<String, Vec<Value>>,
    redo: HashMap<String, Vec<Value>>,
    rng_state: u64,
    /// Loaded file modules, cached by absolute path (C05 idempotent import).
    modules: HashMap<PathBuf, Rc<RefCell<LoadedModule>>>,
    /// Active import chain for circular-import detection.
    import_stack: Vec<(PathBuf, String)>,
    /// Directory relative imports resolve against (the importing file's dir).
    cur_dir: Option<PathBuf>,
    /// Cached native stdlib module values.
    stdlib_cache: HashMap<String, Value>,
}

impl Vm {
    pub fn new() -> Self {
        Vm::default()
    }

    pub fn set_base_dir(&mut self, dir: PathBuf) {
        self.cur_dir = Some(dir);
    }

    pub fn take_output(&mut self) -> String {
        std::mem::take(&mut self.out)
    }

    pub fn run_program(&mut self, prog: Rc<Program>) -> Result<(), VmError> {
        self.frames.push(Frame {
            prog,
            func_idx: 0,
            ip: 0,
            locals: None,
            iters: vec![],
            handlers: vec![],
        });
        self.exec_until_depth(0)
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, VmError> {
        let func = Func {
            name: "<expr>".into(),
            params: vec![],
            names: vec![],
            chunk: chunk.clone(),
        };
        let prog = Rc::new(Program { funcs: vec![func], env: Default::default() });
        self.run_program(prog)?;
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
                    let fprog = self.frames.last().unwrap().prog.clone();
                    self.enter_func(fprog, fidx as usize, args)?;
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
                    let fprog = self.frames.last().unwrap().prog.clone();
                    self.enter_func(fprog, fidx, args)?;
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
                                arith_err("plus", &cur, &delta, e)
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
                        // C05: reading a member of an imported module — its
                        // top-level variables live in the module's env.
                        Value::Module(ModuleVal::Loaded(lm)) => {
                            let (path, val, known) = {
                                let lm = lm.borrow();
                                let v = lm.prog.env.borrow().get(&name).cloned();
                                let mut names: Vec<String> =
                                    lm.prog.env.borrow().keys().cloned().collect();
                                names.extend(lm.prog.funcs.iter().map(|f| f.name.clone()));
                                names.retain(|n| n != "<main>");
                                names.sort();
                                (lm.path.clone(), v, names)
                            };
                            match val {
                                Some(v) => self.stack.push(v),
                                None => {
                                    let hint = suggest_hint(&name, &known.iter().map(|s| s.as_str()).collect::<Vec<_>>());
                                    return Err(err(messages::modules::module_no_member(
                                        &path,
                                        &name,
                                        &hint,
                                        &known.join(", "),
                                    )));
                                }
                            }
                        }
                        Value::Nothing => {
                            return Err(signal_err(messages::interp::field_of_nothing(&name)));
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
                    if matches!(v, Value::Module(_)) {
                        return Err(err(format!(
                            "'a copy of' cannot copy {} — a module/function is not a value",
                            render(&v)
                        )));
                    }
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
                Instr::TryPush(catch_ip, mode) => {
                    let iters_len = self.frames.last().unwrap().iters.len();
                    self.frames
                        .last_mut()
                        .unwrap()
                        .handlers
                        .push((catch_ip, self.stack.len(), iters_len, mode));
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
                Instr::ItemAt => {
                    let hay = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let needle_idx = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let idx = as_i64(&needle_idx).ok_or_else(|| {
                        err(messages::interp::item_needs_num_index())
                    })?;
                    match &hay {
                        Value::List(items) => {
                            let len = items.borrow().len() as i64;
                            if idx < 1 || idx > len {
                                return Err(err(messages::interp::item_out_of_bounds(idx, len)));
                            }
                            let v = items.borrow()[idx as usize - 1].clone();
                            self.stack.push(v);
                        }
                        Value::Text(t) => {
                            let chars: Vec<char> = t.chars().collect();
                            let len = chars.len() as i64;
                            if idx < 1 || idx > len {
                                return Err(err(messages::interp::text_at_oob(idx, len)));
                            }
                            self.stack.push(Value::Text(chars[idx as usize - 1].to_string()));
                        }
                        _ => return Err(err(messages::interp::item_needs_list())),
                    }
                }
                Instr::FirstItem | Instr::LastItem => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let out = match &v {
                        Value::List(items) => {
                            let items = items.borrow();
                            if items.is_empty() {
                                Value::Nothing
                            } else if matches!(instr, Instr::FirstItem) {
                                items[0].clone()
                            } else {
                                items[items.len() - 1].clone()
                            }
                        }
                        _ => Value::Nothing,
                    };
                    self.stack.push(out);
                }
                Instr::CountOf => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let n = match &v {
                        Value::List(items) => items.borrow().len(),
                        Value::Text(t) => t.chars().count(),
                        _ => return Err(err(messages::interp::count_needs_sized())),
                    };
                    self.stack.push(Value::Int(n.into()));
                }
                Instr::NumVal => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let out = match &v {
                        Value::Int(_) | Value::Float(_) => v.clone(),
                        Value::Text(t) => {
                            let s = t.trim();
                            if let Ok(i) = s.parse::<num_bigint::BigInt>() {
                                Value::Int(i)
                            } else if let Ok(f) = s.parse::<f64>() {
                                Value::Float(f)
                            } else {
                                Value::Nothing
                            }
                        }
                        _ => Value::Nothing,
                    };
                    self.stack.push(out);
                }
                Instr::RandomBetween => {
                    let b = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let a = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let (ai, bi) = match (&a, &b) {
                        (Value::Int(x), Value::Int(y)) => (as_i64(&Value::Int(x.clone())), as_i64(&Value::Int(y.clone()))),
                        _ => (None, None),
                    };
                    let (Some(ai), Some(bi)) = (ai, bi) else {
                        return Err(err(messages::interp::random_needs_nums()));
                    };
                    let (lo, hi) = if ai <= bi { (ai, bi) } else { (bi, ai) };
                    let span = (hi - lo + 1).max(1) as u64;
                    self.rng_state ^= self.rng_state << 13;
                    self.rng_state ^= self.rng_state >> 7;
                    self.rng_state ^= self.rng_state << 17;
                    let roll = (self.rng_state >> 11) % span;
                    self.stack.push(Value::Int((lo as u64 + roll).into()));
                }
                Instr::ToText => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    self.stack.push(Value::Text(render(&v)));
                }
                Instr::PushNothing => {
                    self.stack.push(Value::Nothing);
                }
                Instr::UseStdLib { text } => {
                    let text = prog.funcs[fi].names[text as usize].clone();
                    let lib = parse_use_text(&text)
                        .ok_or_else(|| err(messages::stdlib::use_form(&text)))?;
                    if !STDLIBS.contains(&lib.as_str()) {
                        let mut libs = STDLIBS.to_vec();
                        libs.sort_unstable();
                        return Err(err(messages::stdlib::unknown_lib(&lib, &libs.join(", "))));
                    }
                    let v = self.stdlib_value(&lib);
                    self.cur_env().borrow_mut().insert(lib, v);
                }
                Instr::UseModule { name, path, line } => {
                    let bind = prog.funcs[fi].names[name as usize].clone();
                    let path_s = prog.funcs[fi].names[path as usize].clone();
                    let v = self.load_module(&path_s, line as usize)?;
                    self.cur_env().borrow_mut().insert(bind, v);
                }
                Instr::ModuleCall { module, func, argc } => {
                    let mname = prog.funcs[fi].names[module as usize].clone();
                    let fname = prog.funcs[fi].names[func as usize].clone();
                    let at = self.stack.len() - argc as usize;
                    let args: Vec<Value> = self.stack.split_off(at);
                    let mval = self.load_name(fi, &mname)?;
                    match mval {
                        Value::Module(ModuleVal::Native(nm)) => {
                            let v = self.call_native(&nm.name, &fname, &args)?;
                            self.stack.push(v);
                        }
                        Value::Module(ModuleVal::Loaded(lm)) => {
                            let fidx = {
                                let lm = lm.borrow();
                                match lm.prog.funcs.iter().position(|f| f.name == fname) {
                                    Some(i) => i,
                                    None => {
                                        let names: Vec<&str> = lm
                                            .prog
                                            .funcs
                                            .iter()
                                            .map(|f| f.name.as_str())
                                            .collect();
                                        let hint = suggest_hint(&fname, &names);
                                        let call = format!("{mname}.{fname}(...)");
                                        return Err(err(messages::modules::module_no_function(
                                            &lm.path, &fname, &hint, &call,
                                        )));
                                    }
                                }
                            };
                            let params = lm.borrow().prog.funcs[fidx].params.len();
                            if params != args.len() {
                                let hint_params =
                                    lm.borrow().prog.funcs[fidx].params.join(" and ");
                                return Err(err(messages::interp::func_arity(
                                    &fname,
                                    params,
                                    args.len(),
                                    &hint_params,
                                )));
                            }
                            let fprog = lm.borrow().prog.clone();
                            self.enter_func(fprog, fidx, args)?;
                        }
                        other => {
                            return Err(err(messages::modules::not_a_module(other.type_name())));
                        }
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

    /// Runs bytecode until the frame stack shrinks back to `depth`. Module
    /// loading pushes an isolated base frame and recurses through here.
    fn exec_until_depth(&mut self, depth: usize) -> Result<(), VmError> {
        loop {
            if self.frames.len() <= depth {
                return Ok(());
            }
            let (prog, fi, ip) = {
                let f = self.frames.last().unwrap();
                (f.prog.clone(), f.func_idx, f.ip)
            };
            let code_len = prog.funcs[fi].chunk.code.len();
            if ip >= code_len {
                // A base frame ran to its end — pop back to the caller's level.
                self.frames.truncate(depth);
                return Ok(());
            }
            let instr = prog.funcs[fi].chunk.code[ip].clone();
            self.frames.last_mut().unwrap().ip += 1;
            if let Err(e) = self.step(&prog, instr) {
                let msg = e.msg;
                let is_signal = e.signal;
                let mut handled = false;
                loop {
                    if self.frames.len() <= depth {
                        break;
                    }
                    let handler = self.frames.last().and_then(|f| f.handlers.last().copied());
                    match handler {
                        Some((catch_ip, stack_len, iters_len, mode)) => {
                            if mode == 2 && !is_signal {
                                self.frames.last_mut().unwrap().handlers.pop();
                                continue;
                            }
                            self.frames.last_mut().unwrap().handlers.pop();
                            self.stack.truncate(stack_len);
                            let fr = self.frames.last_mut().unwrap();
                            fr.iters.truncate(iters_len);
                            fr.ip = catch_ip as usize;
                            if mode == 0 {
                                self.stack.push(Value::Text(msg.clone()));
                            } else if mode == 2 {
                                self.stack.push(Value::Nothing);
                            }
                            handled = true;
                        }
                        None => {
                            self.frames.pop();
                        }
                    }
                    if handled {
                        break;
                    }
                }
                if !handled {
                    // No handler above `depth` — surface to the outer driver.
                    self.frames.truncate(depth);
                    return Err(VmError { msg, signal: false });
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

    fn cur_env(&self) -> Env {
        self.frames.last().unwrap().prog.env.clone()
    }

    fn load_name(&self, _fi: usize, name: &str) -> Result<Value, VmError> {
        if let Some(locals) = &self.frames.last().unwrap().locals {
            if let Some(v) = locals.get(name) {
                return Ok(v.clone());
            }
        }
        let env = self.frames.last().unwrap().prog.env.clone();
        if let Some(v) = env.borrow().get(name) {
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
        let env = self.cur_env();
        if env.borrow().contains_key(&name) || self.is_main(fi) {
            env.borrow_mut().insert(name, v);
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
        let env = self.frames.last().unwrap().prog.env.clone();
        if env.borrow().contains_key(name) {
            env.borrow_mut().insert(name.to_string(), v);
            return Ok(());
        }
        Err(err(messages::interp::var_not_found(name)))
    }

    fn enter_func(&mut self, prog: Rc<Program>, fidx: usize, args: Vec<Value>) -> Result<(), VmError> {
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
        self.frames.push(Frame {
            prog: prog.clone(),
            func_idx: fidx,
            ip: 0,
            locals: Some(locals),
            iters: vec![],
            handlers: vec![],
        });
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
            Err(e) => Err(arith_err(op, &a, &b, e)),
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
        Value::Float(f) => {
            // Oracle rule: integral floats print without the decimal point.
            if f.fract() == 0.0 && f.is_finite() {
                format!("{}", *f as i128)
            } else {
                crate::lexer::fmt_float(*f)
            }
        }
        Value::Text(t) => t.clone(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Nothing => "nothing".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.borrow().iter().map(render).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Thing(t) => format!("{}(...)", t.borrow().cls),
        Value::Dict(d) => py_repr_dict(&d.borrow()),
        Value::Module(m) => match m {
            ModuleVal::Native(n) => format!("module {}(...)", n.name),
            ModuleVal::Loaded(l) => format!("module {}(...)", l.borrow().path),
        },
    }
}

/// Python-repr rendering: used whenever a dictionary is (or contains) the value,
/// because the oracle prints dicts via Python's str()/repr() rules.
fn py_repr_dict(d: &indexmap::IndexMap<String, Value>) -> String {
    let inner: Vec<String> = d.iter().map(|(k, v)| format!("'{}': {}", k, py_repr(v))).collect();
    format!("{{{}}}", inner.join(", "))
}

fn py_repr(v: &Value) -> String {
    match v {
        Value::Text(t) => format!("'{}'", t),
        Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        Value::Nothing => "None".to_string(),
        Value::List(items) => {
            let inner: Vec<String> = items.borrow().iter().map(py_repr).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Dict(d) => py_repr_dict(&d.borrow()),
        Value::Float(f) => {
            // Python str() keeps a decimal point for integral floats.
            if f.fract() == 0.0 && f.is_finite() {
                format!("{:.1}", f)
            } else {
                crate::lexer::fmt_float(*f)
            }
        }
        other => render(other),
    }
}

impl Vm {
    /// Public handle so hosts (e.g. `nova test`) can pre-bind a stdlib
    /// namespace without requiring an explicit `use` line in user files.
    pub fn stdlib_module(&mut self, name: &str) -> Value {
        self.stdlib_value(name)
    }

    fn stdlib_value(&mut self, name: &str) -> Value {
        if let Some(v) = self.stdlib_cache.get(name) {
            return v.clone();
        }
        let v = Value::Module(ModuleVal::Native(Rc::new(NativeModule { name: name.to_string() })));
        self.stdlib_cache.insert(name.to_string(), v.clone());
        v
    }

    fn rng_next(&mut self) -> u64 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        self.rng_state
    }

    /// C05 loader: cache-first, cycle-detecting, fully isolated env per module.
    fn load_module(&mut self, path_src: &str, line: usize) -> Result<Value, VmError> {
        let base = self.cur_dir.clone().unwrap_or_else(|| PathBuf::from("."));
        let ap: PathBuf = base.join(path_src);
        if let Some(m) = self.modules.get(&ap) {
            return Ok(Value::Module(ModuleVal::Loaded(m.clone())));
        }
        if self.import_stack.iter().any(|(p, _)| *p == ap) {
            let mut chain = self
                .import_stack
                .iter()
                .map(|(_, n)| n.clone())
                .collect::<Vec<_>>()
                .join(" \u{2192} ");
            chain.push_str(&format!(" \u{2192} {path_src}"));
            return Err(err(messages::modules::circular_import(&chain)));
        }
        let src = std::fs::read_to_string(&ap).map_err(|_| {
            err(messages::modules::module_file_not_found(
                path_src,
                &base.to_string_lossy(),
            ))
        })?;
        let stmts = crate::parser::parse_source(&src).map_err(|e| {
            err(format!("{} (in module '{}')", e.msg, path_src))
        })?;
        if stmts.iter().any(|s| matches!(s.kind, SKind::WhenProgramStarts { .. })) {
            return Err(err(messages::modules::no_mains()));
        }
        let prog = Rc::new(crate::compiler::compile_program(&stmts).map_err(|ce| {
            err(format!(
                "line {line}: this feature is not available in the native preview yet ({}) \u{2014} the Python bootstrap can run it",
                ce.kind
            ))
        })?);

        let dir = ap.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
        let saved_dir = self.cur_dir.clone();
        let depth = self.frames.len();
        self.import_stack.push((ap.clone(), path_src.to_string()));
        self.cur_dir = Some(dir);
        self.frames.push(Frame {
            prog: prog.clone(),
            func_idx: 0,
            ip: 0,
            locals: None,
            iters: vec![],
            handlers: vec![],
        });
        let run_res = self.exec_until_depth(depth);
        self.cur_dir = saved_dir;
        self.import_stack.pop();
        run_res?;

        let lm = Rc::new(RefCell::new(LoadedModule { prog, path: path_src.to_string() }));
        self.modules.insert(ap, lm.clone());
        Ok(Value::Module(ModuleVal::Loaded(lm)))
    }

    fn call_native(&mut self, module: &str, fname: &str, args: &[Value]) -> Result<Value, VmError> {
        match (module, fname) {
            ("json", "parse") => {
                let v = only_arg(args)?;
                let s = as_text("parse", v)?;
                match serde_json::from_str::<serde_json::Value>(&s) {
                    Ok(j) => Ok(Value::dict_from_json(&j)),
                    Err(e) => Err(err(messages::stdlib::json_parse_invalid(e.line()))),
                }
            }
            ("json", "stringify") => {
                let v = only_arg(args)?;
                Ok(Value::Text(
                    serde_json::to_string(&to_serde(v))
                        .map_err(|e| err(format!("cannot stringify: {e}")))?,
                ))
            }
            ("file", "read") => {
                let path = as_text("read", only_arg(args)?)?;
                std::fs::read_to_string(&path).map(Value::Text).map_err(|e| {
                    let k = e.kind();
                    if k == std::io::ErrorKind::NotFound {
                        err(messages::stdlib::missing_file(&path))
                    } else if k == std::io::ErrorKind::InvalidData {
                        err(messages::stdlib::not_utf8(&path))
                    } else {
                        err(messages::stdlib::cannot_read(&path, &e.to_string()))
                    }
                })
            }
            ("file", "exists") => {
                let path = as_text("exists", only_arg(args)?)?;
                Ok(Value::Bool(Path::new(&path).exists()))
            }
            ("file", "write") => {
                let path = as_text("write", arg(args, 0)?)?;
                let content = as_text("write", arg(args, 1)?)?;
                std::fs::write(&path, content)
                    .map(|_| Value::Nothing)
                    .map_err(|e| err(messages::stdlib::cannot_save(&path, &e.to_string())))
            }
            ("random", "between") => {
                let a = arg(args, 0)?;
                let b = arg(args, 1)?;
                let (Some(ai), Some(bi)) = (as_i64(a), as_i64(b)) else {
                    return Err(err(messages::stdlib::random_between_needs_nums()));
                };
                let (lo, hi) = if ai <= bi { (ai, bi) } else { (bi, ai) };
                let span = (hi - lo + 1).max(1) as u64;
                let roll = self.rng_next() % span;
                Ok(Value::Int((lo as u64 + roll).into()))
            }
            ("random", "pick") => {
                let v = only_arg(args)?.clone();
                let xs = as_list("pick", &v)?;
                if xs.is_empty() {
                    return Err(err(messages::stdlib::random_pick_needs_list()));
                }
                let i = (self.rng_next() % xs.len() as u64) as usize;
                Ok(xs[i].clone())
            }
            ("random", "shuffle") => {
                let v = only_arg(args)?.clone();
                let mut out = as_list("shuffle", &v)?.clone();
                for i in (1..out.len()).rev() {
                    let j = (self.rng_next() % (i as u64 + 1)) as usize;
                    out.swap(i, j);
                }
                Ok(Value::List(Rc::new(RefCell::new(out))))
            }
            ("time", "now") => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                Ok(Value::Float(now.as_secs_f64()))
            }
            ("time", "sleep") => {
                let v = only_arg(args)?;
                let Some(s) = v.as_f64() else {
                    return Err(err(messages::stdlib::time_sleep_needs_num()));
                };
                if s < 0.0 {
                    return Err(err(messages::stdlib::time_sleep_negative()));
                }
                std::thread::sleep(std::time::Duration::from_secs_f64(s));
                Ok(Value::Nothing)
            }
            ("math", "sqrt") => {
                let v = only_arg(args)?;
                let Some(f) = v.as_f64() else {
                    return Err(err(messages::stdlib::math_sqrt_negative()));
                };
                if f < 0.0 {
                    return Err(err(messages::stdlib::math_sqrt_negative()));
                }
                Ok(Value::Float(f.sqrt()))
            }
            ("math", "round") => math_float_to_int(fname_of(args), args, |f| f.round()),
            ("math", "abs") => math_abs(args),
            ("math", "floor") => math_float_to_int(fname_of(args), args, |f| f.floor()),
            ("math", "ceil") => math_float_to_int(fname_of(args), args, |f| f.ceil()),
            ("math", "pow") => {
                let b = arg(args, 0)?;
                let e = arg(args, 1)?;
                let (Some(bf), Some(ef)) = (b.as_f64(), e.as_f64()) else {
                    return Err(err(messages::stdlib::math_pow_needs_nums()));
                };
                Ok(Value::Float(bf.powf(ef)))
            }
            ("text", "upper") | ("text", "lower") | ("text", "trim") => {
                let s = as_text(fname, only_arg(args)?)?;
                let out = match fname {
                    "upper" => s.to_uppercase(),
                    "lower" => s.to_lowercase(),
                    _ => s.trim().to_string(),
                };
                Ok(Value::Text(out))
            }
            ("text", "split") => {
                let s = as_text("split", arg(args, 0)?)?;
                let sep = as_text("split", arg(args, 1)?)?;
                if sep.is_empty() {
                    return Err(err(messages::stdlib::text_split_empty_sep()));
                }
                Ok(Value::List(Rc::new(RefCell::new(
                    s.split(sep.as_str()).map(|p| Value::Text(p.to_string())).collect(),
                ))))
            }
            ("text", "join") => {
                let v = only_arg(args)?.clone();
                let sep = as_text("join", arg(args, 1)?)?;
                let parts: Vec<String> =
                    as_list("join", &v)?.iter().map(render).collect();
                Ok(Value::Text(parts.join(&sep)))
            }
            ("text", "replace") => {
                let s = as_text("replace", arg(args, 0)?)?;
                let from = as_text("replace", arg(args, 1)?)?;
                let to = as_text("replace", arg(args, 2)?)?;
                if from.is_empty() {
                    return Err(err(messages::stdlib::text_replace_empty_search()));
                }
                Ok(Value::Text(s.replace(from.as_str(), to.as_str())))
            }
            ("text", "length") => {
                let s = as_text("length", only_arg(args)?)?;
                Ok(Value::Int(s.chars().count().into()))
            }
            ("text", "contains") => {
                let s = as_text("contains", arg(args, 0)?)?;
                let sub = as_text("contains", arg(args, 1)?)?;
                Ok(Value::Bool(s.contains(sub.as_str())))
            }
            ("text", "at") => {
                let s = as_text("at", arg(args, 0)?)?;
                let n = need_num_i64(arg(args, 1)?)?;
                let chars: Vec<char> = s.chars().collect();
                let size = chars.len() as i64;
                if n < 1 || n > size {
                    return Err(err(messages::stdlib::text_at_out_of_bounds(n, size)));
                }
                Ok(Value::Text(chars[n as usize - 1].to_string()))
            }
            ("text", "slice") => {
                let s = as_text("slice", arg(args, 0)?)?;
                let a = need_num_i64(arg(args, 1)?)?;
                let b = need_num_i64(arg(args, 2)?)?;
                let size = s.chars().count() as i64;
                if a < 1 || b > size || a > b {
                    return Err(err(messages::stdlib::text_slice_out_of_bounds(a, b, size)));
                }
                let out: String =
                    s.chars().skip((a - 1) as usize).take((b - a + 1) as usize).collect();
                Ok(Value::Text(out))
            }
            ("list", "sort") => {
                let v = only_arg(args)?.clone();
                let xs = as_list("sort", &v)?.clone();
                if xs.iter().all(|v| v.is_number()) {
                    let mut out = xs;
                    out.sort_by(|a, b| num_cmp(a, b).unwrap_or(Ordering::Equal));
                    return Ok(Value::List(Rc::new(RefCell::new(out))));
                }
                if xs.iter().all(|v| matches!(v, Value::Text(_))) {
                    let mut out = xs;
                    out.sort_by(|a, b| match (a, b) {
                        (Value::Text(x), Value::Text(y)) => x.cmp(y),
                        _ => Ordering::Equal,
                    });
                    return Ok(Value::List(Rc::new(RefCell::new(out))));
                }
                Err(err(messages::stdlib::list_sort_mixed(&type_set(&xs))))
            }
            ("list", "reverse") => {
                let v = only_arg(args)?.clone();
                let mut xs = as_list("reverse", &v)?.clone();
                xs.reverse();
                Ok(Value::List(Rc::new(RefCell::new(xs))))
            }
            ("list", "min") | ("list", "max") => {
                let v = only_arg(args)?.clone();
                let xs = as_list(fname, &v)?.clone();
                if xs.is_empty() {
                    return Err(err(messages::stdlib::list_min_max_empty(fname)));
                }
                if !xs.iter().all(|x| x.is_number()) {
                    return Err(err(messages::stdlib::list_min_max_needs_nums(fname)));
                }
                let mut best = 0usize;
                for (i, x) in xs.iter().enumerate() {
                    let ord = num_cmp(x, &xs[best]).unwrap_or(Ordering::Equal);
                    if (fname == "max" && ord == Ordering::Greater)
                        || (fname != "max" && ord == Ordering::Less)
                    {
                        best = i;
                    }
                }
                Ok(xs[best].clone())
            }
            ("list", "keys") => {
                let v = only_arg(args)?;
                match v {
                    Value::Dict(d) => {
                        let keys: Vec<String> =
                            d.borrow().keys().cloned().collect::<Vec<_>>();
                        let mut sorted = keys;
                        sorted.sort();
                        Ok(Value::List(Rc::new(RefCell::new(
                            sorted.into_iter().map(Value::Text).collect(),
                        ))))
                    }
                    other => {
                        Err(err(messages::stdlib::list_keys_needs_dict(&render(other))))
                    }
                }
            }
            ("list", "values") => {
                let v = only_arg(args)?;
                match v {
                    Value::Dict(d) => {
                        let d = d.borrow();
                        let mut keys: Vec<&String> = d.keys().collect();
                        keys.sort();
                        let vals: Vec<Value> =
                            keys.iter().map(|k| d[k.as_str()].clone()).collect();
                        Ok(Value::List(Rc::new(RefCell::new(vals))))
                    }
                    other => {
                        Err(err(messages::stdlib::list_values_needs_dict(&render(other))))
                    }
                }
            }
            ("test", "equal") => {
                let a = arg(args, 0)?;
                let b = arg(args, 1)?;
                if nova_eq(a, b) {
                    Ok(Value::Nothing)
                } else {
                    Err(err(messages::test_runner::equal_failed(&render(b), &render(a))))
                }
            }
            ("test", "true") => {
                let v = only_arg(args)?;
                match v {
                    Value::Bool(true) => Ok(Value::Nothing),
                    other => Err(err(messages::test_runner::true_failed(&render(other)))),
                }
            }
            ("test", "fail") => {
                let msg = match args.first() {
                    Some(v) => render(v),
                    None => "explicit failure".to_string(),
                };
                Err(err(messages::test_runner::explicit_fail(&msg)))
            }

            _ => Err(err(format!(
                "module '{module}' has no function '{fname}' — call: {module}.{fname}(...)"
            ))),
        }
    }
}

fn fname_of(_args: &[Value]) -> &'static str {
    ""
}

fn only_arg(args: &[Value]) -> Result<&Value, VmError> {
    args.first().ok_or_else(|| err("stack underflow".into()))
}

fn arg(args: &[Value], i: usize) -> Result<&Value, VmError> {
    args.get(i).ok_or_else(|| err("stack underflow".into()))
}

fn as_text(fn_name: &str, v: &Value) -> Result<String, VmError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(err(messages::stdlib::text_needs_text(fn_name, &render(other)))),
    }
}

fn as_list<'a>(fn_name: &str, v: &'a Value) -> Result<std::cell::Ref<'a, Vec<Value>>, VmError> {
    match v {
        Value::List(items) => Ok(items.borrow()),
        other => Err(err(messages::stdlib::list_needs_list(fn_name, &render(other)))),
    }
}

fn need_num_i64(v: &Value) -> Result<i64, VmError> {
    as_i64(v).ok_or_else(|| err(messages::stdlib::text_at_needs_num()))
}

fn math_float_to_int(
    _fname: &str,
    args: &[Value],
    f: impl Fn(f64) -> f64,
) -> Result<Value, VmError> {
    let v = only_arg(args)?;
    let Some(x) = v.as_f64() else {
        // The exact function name is recovered by the caller's match arm; the
        // message shape is identical either way.
        return Err(err(messages::stdlib::math_needs_num("round")));
    };
    Ok(Value::Int((f(x) as i64).into()))
}

fn math_abs(args: &[Value]) -> Result<Value, VmError> {
    let v = only_arg(args)?;
    match v {
        Value::Int(i) => Ok(Value::Int(if i.sign() == num_bigint::Sign::Minus {
            -i.clone()
        } else {
            i.clone()
        })),
        Value::Float(f) => Ok(Value::Float(f.abs())),
        _ => Err(err(messages::stdlib::math_needs_num("abs"))),
    }
}

fn type_set(xs: &[Value]) -> String {
    let mut names: Vec<&str> = xs
        .iter()
        .map(|v| match v {
            Value::Int(_) | Value::Float(_) => "number",
            Value::Text(_) => "text",
            other => other.type_name(),
        })
        .collect();
    names.sort_unstable();
    names.dedup();
    names.join(", ")
}

/// Mirrors the oracle's did-you-mean (difflib cutoff 0.6, best single match).
fn suggest_hint(name: &str, candidates: &[&str]) -> String {
    let mut best: Option<(usize, &str)> = None;
    for c in candidates {
        if *c == name {
            continue;
        }
        let d = levenshtein(name, c);
        let limit = ((name.len().max(c.len()) as f64) * 0.4).ceil() as usize;
        if d <= limit && best.map(|(bd, _)| d < bd).unwrap_or(true) {
            best = Some((d, c));
        }
    }
    match best {
        Some((_, c)) => format!(" \u{2014} did you mean '{c}'?"),
        None => String::new(),
    }
}

fn parse_use_text(text: &str) -> Option<String> {
    let mut ws = text.split_whitespace().map(|w| w.to_lowercase()).collect::<Vec<_>>();
    if ws.first().map(|w| w == "the").unwrap_or(false) {
        ws.remove(0);
    }
    let head = ws.first()?.clone();
    if head != "standard" {
        return None;
    }
    ws.remove(0);
    if ws.last().map(|w| w == "library").unwrap_or(false) {
        ws.pop();
    }
    if ws.len() != 1 {
        return None;
    }
    Some(ws.remove(0))
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn to_serde(v: &Value) -> serde_json::Value {
    match v {
        Value::Nothing => serde_json::Value::Null,
        Value::Bool(b) => serde_json::Value::Bool(*b),
        Value::Int(i) => {
            let s = i.to_string();
            let num = match s.parse::<i64>() {
                Ok(n) => serde_json::Number::from(n),
                Err(_) => match s.parse::<u64>() {
                    Ok(n) => serde_json::Number::from(n),
                    Err(_) => serde_json::Number::from_f64(s.parse().unwrap_or(0.0))
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                },
            };
            serde_json::Value::Number(num)
        }
        Value::Float(f) => match serde_json::Number::from_f64(*f) {
            Some(n) => serde_json::Value::Number(n),
            None => serde_json::Value::Null,
        },
        Value::Text(t) => serde_json::Value::String(t.clone()),
        Value::List(items) => {
            serde_json::Value::Array(items.borrow().iter().map(to_serde).collect())
        }
        Value::Dict(d) => {
            let mut m = serde_json::Map::new();
            for (k, val) in d.borrow().iter() {
                m.insert(k.clone(), to_serde(val));
            }
            serde_json::Value::Object(m)
        }
        other => serde_json::Value::String(render(other)),
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

fn arith_err(_op: &str, _a: &Value, _b: &Value, e: ArithError) -> VmError {
    let msg = match &e {
        ArithError::TypeMismatch { left, right } => {
            messages::interp::plus_type_mismatch(left, right)
        }
        ArithError::DivByZero => messages::interp::div_by_zero(),
        ArithError::ModByZero => messages::interp::mod_by_zero(),
        ArithError::OnNothing => messages::interp::arith_on_nothing(),
    };
    VmError { msg, signal: matches!(e, ArithError::OnNothing) }
}










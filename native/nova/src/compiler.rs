use crate::ast::{EKind, ENode, SBlock, SKind, SNode};
use crate::bytecode::{Chunk, Func, Instr, Program};

pub struct CompileError {
    pub kind: &'static str,
}

type R = Result<(), CompileError>;

struct LoopCtx {
    continue_ip: u16,
    break_jumps: Vec<u16>,
}

struct Builder {
    func: Func,
    loops: Vec<LoopCtx>,
    pending_ensures: Vec<ENode>,
}

impl Builder {
    fn emit(&mut self, i: Instr) {
        self.func.chunk.code.push(i);
    }

    fn here(&self) -> u16 {
        self.func.chunk.code.len() as u16
    }

    fn name_index(&mut self, name: &str) -> u16 {
        self.func.name_index(name)
    }
}

pub struct Compiler {
    builders: Vec<Builder>,
    cur: usize,
}

pub fn compile_program(stmts: &[SNode]) -> Result<Program, CompileError> {
    let stmts: Vec<SNode> = if stmts.len() == 1 {
        match &stmts[0].kind {
            SKind::WhenProgramStarts { body } => body.stmts.clone(),
            _ => stmts.to_vec(),
        }
    } else {
        stmts.to_vec()
    };

    let mut c = Compiler {
        builders: vec![Builder {
            func: Func { name: "<main>".into(), params: vec![], names: vec![], chunk: Chunk::default() },
            loops: vec![],
            pending_ensures: vec![],
        }],
        cur: 0,
    };

    for def in stmts.iter().filter(|s| matches!(s.kind, SKind::FuncDef { .. } | SKind::ThingDef { .. })) {
        let (ctor_name, is_thing) = match &def.kind {
            SKind::FuncDef { name, .. } => (name.clone(), false),
            SKind::ThingDef { name, .. } => (format!("@new:{name}"), true),
            _ => unreachable!(),
        };
        if is_thing {
            c.builders.push(Builder {
                func: Func {
                    name: ctor_name,
                    params: vec![],
                    names: vec![],
                    chunk: Chunk::default(),
                },
                loops: vec![],
                pending_ensures: vec![],
            });
        } else if let SKind::FuncDef { name, .. } = &def.kind {
            c.builders.push(Builder {
                func: Func {
                    name: name.clone(),
                    params: vec![],
                    names: vec![],
                    chunk: Chunk::default(),
                },
                loops: vec![],
                pending_ensures: vec![],
            });
        }
    }

    for st in stmts {
        c.stmt(&st)?;
    }

    c.push_nothing(0);
    c.builders[0].emit(Instr::Halt);

    Ok(Program { funcs: c.builders.into_iter().map(|b| b.func).collect() })
}

impl Compiler {
    fn b(&mut self) -> &mut Builder {
        &mut self.builders[self.cur]
    }

    fn switch_to_func(&mut self, name: &str) -> usize {
        let idx =
            self.builders.iter().position(|b| b.func.name == name).expect("pre-declared func");
        let saved = self.cur;
        self.cur = idx;
        saved
    }

    fn restore(&mut self, saved: usize) {
        self.cur = saved;
    }

    fn push_nothing(&mut self, fi: usize) {
        let has = self.builders[fi]
            .func
            .chunk
            .consts
            .iter()
            .position(|c| matches!(c, crate::value::Value::Nothing));
        let idx = match has {
            Some(i) => i as u16,
            None => {
                self.builders[fi].func.chunk.consts.push(crate::value::Value::Nothing);
                (self.builders[fi].func.chunk.consts.len() - 1) as u16
            }
        };
        self.builders[fi].emit(Instr::Const(idx));
    }

    fn emit_in_cur(&mut self, i: Instr) {
        self.b().emit(i);
    }

    fn jump_to(&mut self, target: u16) {
        self.emit_in_cur(Instr::Jump(target));
    }

    fn stmt(&mut self, st: &SNode) -> R {
        match &st.kind {
            SKind::Say { exprs, newline } => {
                let n = exprs.len().min(u8::MAX as usize) as u8;
                for e in exprs {
                    self.expr(e)?;
                }
                self.emit_in_cur(Instr::Print(n, *newline));
                Ok(())
            }
            SKind::Assign { target, expr } => match &target.kind {
                EKind::Var(name) => {
                    self.expr(expr)?;
                    let idx = self.b().name_index(name);
                    self.emit_in_cur(Instr::StoreName(idx));
                    Ok(())
                }
                EKind::Field { obj, name } => {
                    self.expr(obj)?;
                    self.expr(expr)?;
                    let idx = self.b().name_index(name);
                    self.emit_in_cur(Instr::StoreField(idx));
                    Ok(())
                }
                _ => unreachable!("parser lvalues"),
            },
            SKind::AddTo { name, expr } => {
                self.expr(expr)?;
                let idx = self.b().name_index(name);
                self.emit_in_cur(Instr::AddToName(idx));
                Ok(())
            }
            SKind::TakeFrom { .. } => Err(CompileError { kind: "TakeFrom (N04)" }),
            SKind::ExprStmt { expr } => {
                self.expr(expr)?;
                self.emit_in_cur(Instr::Pop);
                Ok(())
            }
            SKind::Contract { kind, expr } => {
                if *kind == "requires" {
                    self.expr(expr)?;
                    self.emit_in_cur(Instr::RequireCheck);
                } else {
                    if self.cur == 0 {
                        return Err(CompileError { kind: "ensures outside function" });
                    }
                    self.b().pending_ensures.push(expr.clone());
                }
                Ok(())
            }
            SKind::TryStmt { body, errname, handler } => match handler {
                Some(hb) => {
                    let tp = self.b().here();
                    self.emit_in_cur(Instr::TryPush(0, true));
                    self.block(body)?;
                    self.emit_in_cur(Instr::TryPop);
                    let skip = self.b().here();
                    self.emit_in_cur(Instr::Jump(0));
                    let catch_ip = self.b().here();
                    self.patch(tp, catch_ip);
                    if let Some(n) = errname {
                        let idx = self.b().name_index(n);
                        self.emit_in_cur(Instr::StoreName(idx));
                    }
                    self.block(hb)?;
                    let end = self.b().here();
                    self.patch(skip, end);
                    Ok(())
                }
                None => {
                    let tp = self.b().here();
                    self.emit_in_cur(Instr::TryPush(0, false));
                    self.block(body)?;
                    self.emit_in_cur(Instr::TryPop);
                    let catch_ip = self.b().here();
                    self.patch(tp, catch_ip);
                    Ok(())
                }
            },
            SKind::Check { subject, arms, otherwise } => {
                self.expr(subject)?;
                let subj_idx = self.b().name_index("@subject");
                self.emit_in_cur(Instr::StoreName(subj_idx));
                let mut end_jumps: Vec<u16> = Vec::new();
                let mut cond_jumps: Vec<u16> = Vec::new();
                for (i, arm) in arms.iter().enumerate() {
                    if i > 0 {
                        let prev = cond_jumps.pop().unwrap();
                        let target = self.b().here();
                        self.patch(prev, target);
                    }
                    let s = subj_idx;
                    match (arm.kind, &arm.val) {
                        ("isnum", _) => {
                            self.emit_in_cur(Instr::LoadName(s));
                            self.emit_in_cur(Instr::IsNumber);
                        }
                        ("isempty", _) => {
                            self.emit_in_cur(Instr::LoadName(s));
                            self.emit_in_cur(Instr::IsEmpty);
                        }
                        ("eq", Some(v)) => {
                            self.emit_in_cur(Instr::LoadName(s));
                            self.expr(v)?;
                            self.emit_in_cur(Instr::Eq);
                        }
                        ("startswith", Some(v)) => {
                            self.emit_in_cur(Instr::LoadName(s));
                            self.expr(v)?;
                            self.emit_in_cur(Instr::StartsWith);
                        }
                        ("endswith", Some(v)) => {
                            self.emit_in_cur(Instr::LoadName(s));
                            self.expr(v)?;
                            self.emit_in_cur(Instr::EndsWith);
                        }
                        ("contains", Some(v)) => {
                            self.emit_in_cur(Instr::LoadName(s));
                            self.expr(v)?;
                            self.emit_in_cur(Instr::Contains);
                        }
                        _ => return Err(CompileError { kind: "check-pattern" }),
                    }
                    if arm.neg {
                        self.emit_in_cur(Instr::Not);
                    }
                    let j = self.b().here();
                    self.emit_in_cur(Instr::JumpIfFalsePop(0));
                    self.block(&arm.body)?;
                    let ej = self.b().here();
                    self.emit_in_cur(Instr::Jump(0));
                    end_jumps.push(ej);
                    cond_jumps.push(j);
                }
                if let Some(prev) = cond_jumps.pop() {
                    let target = self.b().here();
                    self.patch(prev, target);
                }
                if let Some(els) = otherwise {
                    self.block(els)?;
                }
                let end = self.b().here();
                for j in end_jumps {
                    self.patch(j, end);
                }
                Ok(())
            }
            SKind::If { branches, otherwise } => self.if_chain(branches, otherwise.as_ref()),
            SKind::RepeatTimes { count, body } => {
                self.expr(count)?;
                self.emit_in_cur(Instr::IterTimesNew);
                self.loop_layout(body, None)
            }
            SKind::RepeatCounting { var, start, end, body } => {
                self.expr(start)?;
                self.expr(end)?;
                self.emit_in_cur(Instr::IterCountNew);
                self.loop_layout(body, Some(var))
            }
            SKind::RepeatEach { var, iterable, body } => {
                self.expr(iterable)?;
                self.emit_in_cur(Instr::IterNew);
                self.loop_layout(body, Some(var))
            }
            SKind::RepeatUntil { cond, body } => {
                let cont = self.b().here();
                self.b().loops.push(LoopCtx { continue_ip: cont, break_jumps: vec![] });
                self.expr(cond)?;
                let exit_jump = self.b().here();
                self.emit_in_cur(Instr::JumpIfTruePop(0));
                self.block(body)?;
                self.jump_to(cont);
                let end = self.b().here();
                let ctx = self.b().loops.pop().unwrap();
                self.patch(exit_jump, end);
                for b in ctx.break_jumps {
                    self.patch(b, end);
                }
                Ok(())
            }
            SKind::RepeatWhile { cond, body } => {
                let cont = self.b().here();
                self.b().loops.push(LoopCtx { continue_ip: cont, break_jumps: vec![] });
                self.expr(cond)?;
                let exit_jump = self.b().here();
                self.emit_in_cur(Instr::JumpIfFalsePop(0));
                self.block(body)?;
                self.jump_to(cont);
                let end = self.b().here();
                let ctx = self.b().loops.pop().unwrap();
                self.patch(exit_jump, end);
                for b in ctx.break_jumps {
                    self.patch(b, end);
                }
                Ok(())
            }
            SKind::RepeatForever { body } => {
                let cont = self.b().here();
                self.b().loops.push(LoopCtx { continue_ip: cont, break_jumps: vec![] });
                self.block(body)?;
                self.jump_to(cont);
                let end = self.b().here();
                let ctx = self.b().loops.pop().unwrap();
                for b in ctx.break_jumps {
                    self.patch(b, end);
                }
                Ok(())
            }
            SKind::BreakStmt => {
                let at = self.b().here();
                self.emit_in_cur(Instr::Jump(0));
                self.b()
                    .loops
                    .last_mut()
                    .ok_or(CompileError { kind: "break outside loop" })?
                    .break_jumps
                    .push(at);
                Ok(())
            }
            SKind::ContinueStmt => {
                let target = self
                    .b()
                    .loops
                    .last()
                    .ok_or(CompileError { kind: "continue outside loop" })?
                    .continue_ip;
                self.jump_to(target);
                Ok(())
            }
            SKind::StopProgram => {
                self.emit_in_cur(Instr::Halt);
                Ok(())
            }
            SKind::ThingDef { name, fields } => {
                let saved = self.switch_to_func(&format!("@new:{name}"));
                for (_fname, default) in fields {
                    match default {
                        Some(e) => self.expr(e)?,
                        None => self.push_nothing(self.cur),
                    }
                }
                let cls_idx = self.b().name_index(name);
                let field_idxs: Vec<u16> =
                    fields.iter().map(|(n, _)| self.b().name_index(n)).collect();
                self.emit_in_cur(Instr::MakeThing {
                    cls: cls_idx,
                    fields: std::rc::Rc::new(field_idxs),
                });
                self.emit_func_exit()?;
                self.restore(saved);
                Ok(())
            }
            SKind::ReturnStmt { expr } => {
                match expr {
                    Some(e) => self.expr(e)?,
                    None => self.push_nothing(self.cur),
                }
                self.emit_func_exit()?;
                Ok(())
            }
            SKind::FuncDef { name, params, body } => {
                let saved = self.switch_to_func(name);
                self.builders[self.cur].func.params = params.clone();
                for st in &body.stmts {
                    if let SKind::Contract { kind: "requires", expr } = &st.kind {
                        self.expr(expr)?;
                        self.emit_in_cur(Instr::RequireCheck);
                    }
                }
                for st in &body.stmts {
                    if matches!(&st.kind, SKind::Contract { kind: "requires", .. }) {
                        continue;
                    }
                    self.stmt(st)?;
                }
                self.push_nothing(self.cur);
                self.emit_func_exit()?;
                self.restore(saved);
                Ok(())
            }
            other => Err(CompileError { kind: skind_name(other) }),
        }
    }

    fn emit_func_exit(&mut self) -> R {
        let ret_idx = self.b().name_index("@ret");
        self.emit_in_cur(Instr::StoreName(ret_idx));
        let ensures = self.builders[self.cur].pending_ensures.clone();
        for e in &ensures {
            self.expr(e)?;
            self.emit_in_cur(Instr::EnsureCheck);
        }
        self.emit_in_cur(Instr::LoadName(ret_idx));
        self.emit_in_cur(Instr::Ret);
        Ok(())
    }

    fn if_chain(&mut self, branches: &[(ENode, SBlock)], otherwise: Option<&SBlock>) -> R {
        let mut end_jumps: Vec<u16> = Vec::new();
        let mut cond_jumps: Vec<u16> = Vec::new();
        for (i, (cond, body)) in branches.iter().enumerate() {
            if i > 0 {
                let prev = cond_jumps.pop().unwrap();
                let target = self.b().here();
                self.patch(prev, target);
            }
            self.expr(cond)?;
            let j = self.b().here();
            self.emit_in_cur(Instr::JumpIfFalsePop(0));
            self.block(body)?;
            let ej = self.b().here();
            self.emit_in_cur(Instr::Jump(0));
            end_jumps.push(ej);
            cond_jumps.push(j);
        }
        if let Some(prev) = cond_jumps.pop() {
            let target = self.b().here();
            self.patch(prev, target);
        }
        if let Some(els) = otherwise {
            self.block(els)?;
        }
        let end = self.b().here();
        for j in end_jumps {
            self.patch(j, end);
        }
        Ok(())
    }

    fn loop_layout(&mut self, body: &SBlock, var: Option<&String>) -> R {
        let cont = self.b().here();
        self.b().loops.push(LoopCtx { continue_ip: cont, break_jumps: vec![] });
        let iter_next_at = self.b().here();
        self.emit_in_cur(Instr::IterNext(0));
        if let Some(v) = var {
            let idx = self.b().name_index(v);
            self.emit_in_cur(Instr::StoreName(idx));
        }
        self.block(body)?;
        self.jump_to(cont);
        let end = self.b().here();
        let ctx = self.b().loops.pop().unwrap();
        self.patch(iter_next_at, end);
        for b in ctx.break_jumps {
            self.patch(b, end);
        }
        Ok(())
    }

    fn block(&mut self, b: &SBlock) -> R {
        for s in &b.stmts {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn patch(&mut self, at: u16, target: u16) {
        let code = &mut self.builders[self.cur].func.chunk.code;
        match &mut code[at as usize] {
            Instr::Jump(t)
            | Instr::JumpIfFalse(t)
            | Instr::JumpIfTrue(t)
            | Instr::JumpIfFalsePop(t)
            | Instr::JumpIfTruePop(t)
            | Instr::IterNext(t) => *t = target,
            Instr::TryPush(t, _) => *t = target,
            _ => unreachable!("patch target not a jump"),
        }
    }

    fn expr(&mut self, e: &ENode) -> R {
        match &e.kind {
            EKind::Lit(l) => {
                let idx = self.const_index(crate::value::Value::from_lit(l));
                self.emit_in_cur(Instr::Const(idx));
                Ok(())
            }
            EKind::StrLit(s) => {
                let idx = self.const_index(crate::value::Value::Text(s.clone()));
                self.emit_in_cur(Instr::Const(idx));
                Ok(())
            }
            EKind::ListLit(items) => {
                for it in items {
                    self.expr(it)?;
                }
                self.emit_in_cur(Instr::MakeList(items.len().min(u16::MAX as usize) as u16));
                Ok(())
            }
            EKind::EmptyListE => {
                self.emit_in_cur(Instr::MakeList(0));
                Ok(())
            }
            EKind::Var(name) => {
                let idx = self.b().name_index(name);
                self.emit_in_cur(Instr::LoadName(idx));
                Ok(())
            }
            EKind::Call { name, args } => {
                for a in args {
                    self.expr(a)?;
                }
                let idx = self.b().name_index(name);
                let argc = args.len().min(u8::MAX as usize) as u8;
                self.emit_in_cur(Instr::CallName(idx, argc));
                Ok(())
            }
            EKind::Bin { op, l, r } => {
                if *op == "and" || *op == "or" {
                    self.expr(l)?;
                    let jump_at = self.b().here();
                    self.emit_in_cur(if *op == "and" {
                        Instr::JumpIfFalse(0)
                    } else {
                        Instr::JumpIfTrue(0)
                    });
                    self.emit_in_cur(Instr::Pop);
                    self.expr(r)?;
                    self.emit_in_cur(Instr::MustBeBool);
                    let after = self.b().here();
                    let code = &mut self.builders[self.cur].func.chunk.code;
                    match &mut code[jump_at as usize] {
                        Instr::JumpIfFalse(t) | Instr::JumpIfTrue(t) => *t = after,
                        _ => unreachable!(),
                    }
                    return Ok(());
                }
                self.expr(l)?;
                self.expr(r)?;
                let instr = match *op {
                    "plus" => Instr::Add,
                    "minus" => Instr::Sub,
                    "times" => Instr::Mul,
                    "divided" => Instr::Div,
                    "mod" => Instr::Mod,
                    "eq" => Instr::Eq,
                    "ne" => Instr::Ne,
                    "lt" => Instr::Lt,
                    "lte" => Instr::Lte,
                    "gt" => Instr::Gt,
                    "gte" => Instr::Gte,
                    "contains" => Instr::Contains,
                    "startswith" => Instr::StartsWith,
                    "endswith" => Instr::EndsWith,
                    other => return Err(CompileError { kind: binop_name(other) }),
                };
                self.emit_in_cur(instr);
                Ok(())
            }
            EKind::NotE(inner) => {
                self.expr(inner)?;
                self.emit_in_cur(Instr::Not);
                Ok(())
            }
            EKind::NewThing { cls, setters } => {
                let ctor = format!("@new:{cls}");
                if !self.builders.iter().any(|b| b.func.name == ctor) {
                    let idx = self.b().name_index(cls);
                    self.emit_in_cur(Instr::UnknownThing(idx));
                    return Ok(());
                }
                let ctor_idx = self.b().name_index(&ctor);
                self.emit_in_cur(Instr::CallName(ctor_idx, 0));
                for (n, v) in setters {
                    self.emit_in_cur(Instr::Dup);
                    self.expr(v)?;
                    let fidx = self.b().name_index(n);
                    self.emit_in_cur(Instr::StoreField(fidx));
                }
                Ok(())
            }
            EKind::Field { obj, name } => {
                self.expr(obj)?;
                let idx = self.b().name_index(name);
                self.emit_in_cur(Instr::GetField(idx));
                Ok(())
            }
            EKind::CopyOf(inner) => {
                self.expr(inner)?;
                self.emit_in_cur(Instr::CopyOf);
                Ok(())
            }
            other => Err(CompileError { kind: ekind_name(other) }),
        }
    }

    fn const_index(&mut self, v: crate::value::Value) -> u16 {
        let chunk = &mut self.builders[self.cur].func.chunk;
        if let Some(i) = chunk.consts.iter().position(|c| const_same(c, &v)) {
            return i as u16;
        }
        chunk.consts.push(v);
        (chunk.consts.len() - 1) as u16
    }
}

fn const_same(a: &crate::value::Value, b: &crate::value::Value) -> bool {
    use crate::value::Value;
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => matches!((a, b), (Value::Nothing, Value::Nothing)),
    }
}

fn binop_name(op: &str) -> &'static str {
    match op {
        "plus" | "minus" | "times" | "divided" | "mod" | "eq" | "ne" | "lt" | "lte" | "gt"
        | "gte" | "contains" | "startswith" | "endswith" => "Bin",
        _ => "Bin",
    }
}

fn ekind_name(k: &EKind) -> &'static str {
    match k {
        EKind::Lit(_) => "Lit",
        EKind::StrLit(_) => "StrLit",
        EKind::ListLit(_) => "ListLit",
        EKind::EmptyListE => "EmptyListE",
        EKind::Var(_) => "Var",
        EKind::Field { .. } => "Field",
        EKind::Bin { .. } => "Bin",
        EKind::NotE(_) => "NotE",
        EKind::Call { .. } => "Call",
        EKind::ModuleCall { .. } => "ModuleCall",
        EKind::NewThing { .. } => "NewThing",
        EKind::NumVal(_) => "NumVal",
        EKind::EverythingAfter { .. } => "EverythingAfter",
        EKind::CountOf(_) => "CountOf",
        EKind::ItemAt { .. } => "ItemAt",
        EKind::FirstItem(_) => "FirstItem",
        EKind::LastItem(_) => "LastItem",
        EKind::IsEmptyE(_) => "IsEmptyE",
        EKind::HasNoItems(_) => "HasNoItems",
        EKind::ExistsE { .. } => "ExistsE",
        EKind::IsNumberTest { .. } => "IsNumberTest",
        EKind::RandomBetween { .. } => "RandomBetween",
        EKind::ContentsOf { .. } => "ContentsOf",
        EKind::EveryTurnedInto { .. } => "EveryTurnedInto",
        EKind::CopyOf(_) => "CopyOf",
        EKind::AskE(_) => "AskE",
        EKind::QuestionE(_) => "QuestionE",
    }
}

fn skind_name(k: &SKind) -> &'static str {
    match k {
        SKind::Say { .. } => "Say",
        SKind::Assign { .. } => "Assign",
        SKind::AddTo { .. } => "AddTo",
        SKind::TakeFrom { .. } => "TakeFrom",
        SKind::If { .. } => "If",
        SKind::RepeatForever { .. } => "RepeatForever",
        SKind::RepeatUntil { .. } => "RepeatUntil",
        SKind::RepeatWhile { .. } => "RepeatWhile",
        SKind::RepeatTimes { .. } => "RepeatTimes",
        SKind::RepeatEach { .. } => "RepeatEach",
        SKind::RepeatCounting { .. } => "RepeatCounting",
        SKind::BreakStmt => "BreakStmt",
        SKind::ContinueStmt => "ContinueStmt",
        SKind::StopProgram => "StopProgram",
        SKind::PauseProgram => "PauseProgram",
        SKind::Check { .. } => "Check",
        SKind::TryStmt { .. } => "TryStmt",
        SKind::FuncDef { .. } => "FuncDef",
        SKind::ThingDef { .. } => "ThingDef",
        SKind::ReturnStmt { .. } => "ReturnStmt",
        SKind::WaitStmt { .. } => "WaitStmt",
        SKind::UseLib { .. } => "UseLib",
        SKind::UseModule { .. } => "UseModule",
        SKind::TrackStmt { .. } => "TrackStmt",
        SKind::UndoStmt { .. } => "UndoStmt",
        SKind::Contract { .. } => "Contract",
        SKind::RemoveStmt { .. } => "RemoveStmt",
        SKind::StoreJson { .. } => "StoreJson",
        SKind::ExprStmt { .. } => "ExprStmt",
        SKind::WhenProgramStarts { .. } => "WhenProgramStarts",
    }
}




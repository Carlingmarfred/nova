use crate::ast::{EKind, ENode, PyLit};
use crate::value::Value;

pub fn compile_expr(e: &ENode, chunk: &mut Chunk) -> Result<(), CompileError> {
    compile_expr_into(e, chunk)
}

#[derive(Debug, Clone)]
pub enum Instr {
    Const(u16),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Lte,
    Gt,
    Gte,
    Contains,
    StartsWith,
    EndsWith,
    Not,
    MustBeBool,
    JumpIfFalse(u16),
    JumpIfTrue(u16),
    MakeList(u16),
    Pop,
    LoadName(u16),
    StoreName(u16),
    AddToName(u16),
    Jump(u16),
    JumpIfFalsePop(u16),
    JumpIfTruePop(u16),
    IterNew,
    IterTimesNew,
    IterCountNew,
    IterNext(u16),
    IterClose,
    Call(u16, u8),
    CallName(u16, u8),
    Ret,
    Halt,
    Print(u8, bool),
}

#[derive(Debug, Default)]
pub struct Func {
    pub name: String,
    pub params: Vec<String>,
    pub names: Vec<String>,
    pub chunk: Chunk,
}

impl Func {
    pub fn name_index(&mut self, name: &str) -> u16 {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            return i as u16;
        }
        self.names.push(name.to_string());
        (self.names.len() - 1) as u16
    }
}

#[derive(Debug, Default)]
pub struct Program {
    pub funcs: Vec<Func>,
}

#[derive(Debug, Clone, Default)]
pub struct Chunk {
    pub code: Vec<Instr>,
    pub consts: Vec<Value>,
}

impl Chunk {
    fn constant(&mut self, v: Value) -> u16 {
        if let Some(i) = self.consts.iter().position(|c| const_eq(c, &v)) {
            return i as u16;
        }
        self.consts.push(v);
        (self.consts.len() - 1) as u16
    }
}

fn const_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => matches!((a, b), (Value::Nothing, Value::Nothing)),
    }
}

fn lit_value(p: &PyLit) -> Value {
    match p {
        PyLit::Int(s) => Value::Int(s.parse().expect("lexer guarantees bigint-parsable ints")),
        PyLit::Float(f) => Value::Float(*f),
        PyLit::Bool(b) => Value::Bool(*b),
        PyLit::Nothing => Value::Nothing,
    }
}

pub struct CompileError {
    pub kind: &'static str,
}

pub fn compile_expr_into(e: &ENode, chunk: &mut Chunk) -> Result<(), CompileError> {
    match &e.kind {
        EKind::Lit(l) => {
            let c = chunk.constant(lit_value(l));
            chunk.code.push(Instr::Const(c));
            Ok(())
        }
        EKind::StrLit(s) => {
            let c = chunk.constant(Value::Text(s.clone()));
            chunk.code.push(Instr::Const(c));
            Ok(())
        }
        EKind::ListLit(items) => {
            for item in items {
                compile_expr(item, chunk)?;
            }
            chunk.code.push(Instr::MakeList(items.len() as u16));
            Ok(())
        }
        EKind::EmptyListE => {
            chunk.code.push(Instr::MakeList(0));
            Ok(())
        }
        EKind::Bin { op, l, r } => {
            compile_expr(l, chunk)?;
            if *op == "and" || *op == "or" {
                let jump_pos = chunk.code.len();
                chunk
                    .code
                    .push(if *op == "and" { Instr::JumpIfFalse(0) } else { Instr::JumpIfTrue(0) });
                chunk.code.push(Instr::Pop);
                compile_expr(r, chunk)?;
                chunk.code.push(Instr::MustBeBool);
                let after = chunk.code.len() as u16;
                match &mut chunk.code[jump_pos] {
                    Instr::JumpIfFalse(target) | Instr::JumpIfTrue(target) => *target = after,
                    _ => unreachable!(),
                }
                return Ok(());
            }
            compile_expr(r, chunk)?;
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
                other => return Err(CompileError { kind: other }),
            };
            chunk.code.push(instr);
            Ok(())
        }
        EKind::NotE(inner) => {
            compile_expr(inner, chunk)?;
            chunk.code.push(Instr::Not);
            Ok(())
        }
        unsupported => Err(CompileError { kind: variant_name(unsupported) }),
    }
}

fn variant_name(e: &EKind) -> &'static str {
    match e {
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



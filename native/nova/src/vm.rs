use crate::bytecode::{Chunk, Instr};
use crate::messages;
use crate::value::{nova_eq, num_cmp, ArithError, Value};
use std::cell::RefCell;
use std::cmp::Ordering;
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

pub struct Vm {
    stack: Vec<Value>,
}

impl Default for Vm {
    fn default() -> Self {
        Self::new()
    }
}

impl Vm {
    pub fn new() -> Self {
        Vm { stack: Vec::new() }
    }

    pub fn run(&mut self, chunk: &Chunk) -> Result<Value, VmError> {
        let mut ip = 0usize;
        while ip < chunk.code.len() {
            let instr = &chunk.code[ip];
            ip += 1;
            match instr {
                Instr::Const(c) => {
                    let v = chunk.consts[*c as usize].clone();
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
                    let (prefix, s) = self.pop2()?;
                    let res = match (&s, &prefix) {
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
                Instr::Not => {
                    let v = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                    let b = truth(&v)?;
                    self.stack.push(Value::Bool(!b));
                }
                Instr::MustBeBool => {
                    let t = self.stack.last().ok_or_else(|| err("stack underflow".into()))?;
                    truth(t)?;
                }
                Instr::JumpIfFalse(target) => {
                    let t = self.stack.last().ok_or_else(|| err("stack underflow".into()))?;
                    if !truth(t)? {
                        ip = *target as usize;
                    }
                }
                Instr::JumpIfTrue(target) => {
                    let t = self.stack.last().ok_or_else(|| err("stack underflow".into()))?;
                    if truth(t)? {
                        ip = *target as usize;
                    }
                }
                Instr::MakeList(n) => {
                    let at = self.stack.len() - *n as usize;
                    let items = self.stack.split_off(at);
                    self.stack.push(Value::List(Rc::new(RefCell::new(items))));
                }
                Instr::Pop => {
                    self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
                }
            }
        }
        if self.stack.is_empty() {
            Ok(Value::Nothing)
        } else {
            Ok(self.stack[self.stack.len() - 1].clone())
        }
    }

    fn pop2(&mut self) -> Result<(Value, Value), VmError> {
        let b = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
        let a = self.stack.pop().ok_or_else(|| err("stack underflow".into()))?;
        Ok((b, a))
    }

    fn bin_arith(&mut self, op: &'static str) -> Result<(), VmError> {
        let (b, a) = self.pop2()?;
        match crate::value::arith(op, &a, &b) {
            Ok(v) => {
                self.stack.push(v);
                Ok(())
            }
            Err(e) => Err(err(arith_msg(op, &a, &b, e))),
        }
    }
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

fn ordering(a: &Value, b: &Value) -> Result<Ordering, VmError> {
    num_cmp(a, b).ok_or_else(|| err(messages::interp::ordering_needs_numbers(a.type_name())))
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

use num_bigint::{BigInt, Sign};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum Value {
    Int(BigInt),
    Float(f64),
    Text(String),
    Bool(bool),
    Nothing,
    List(Rc<RefCell<Vec<Value>>>),
}

impl Value {
    pub fn int(n: i64) -> Value {
        Value::Int(BigInt::from(n))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) | Value::Float(_) => "number",
            Value::Text(_) => "text",
            Value::Bool(_) => "bool",
            Value::Nothing => "nothing",
            Value::List(_) => "list",
        }
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Float(_))
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => i.to_string().parse().ok(),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
}

pub fn nova_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Nothing, Value::Nothing) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Bool(_), _) | (_, Value::Bool(_)) => false,
        (Value::Int(_), Value::Int(_)) => as_bigint(a) == as_bigint(b),
        (Value::Text(x), Value::Text(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(u, v)| nova_eq(u, v))
        }
        (Value::Nothing, _) | (_, Value::Nothing) => false,
        _ => {
            if a.is_number() && b.is_number() {
                num_cmp(a, b) == Some(Ordering::Equal)
            } else {
                false
            }
        }
    }
}

fn as_bigint(v: &Value) -> BigInt {
    match v {
        Value::Int(i) => i.clone(),
        _ => unreachable!("as_bigint on non-int"),
    }
}

pub fn num_cmp(a: &Value, b: &Value) -> Option<Ordering> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Some(x.cmp(y)),
        _ => {
            let x = a.as_f64()?;
            let y = b.as_f64()?;
            x.partial_cmp(&y)
        }
    }
}

#[derive(Debug)]
pub enum ArithError {
    TypeMismatch { left: &'static str, right: &'static str },
    DivByZero,
    ModByZero,
    OnNothing,
}

pub fn arith(op: &str, a: &Value, b: &Value) -> Result<Value, ArithError> {
    if matches!(a, Value::Nothing) || matches!(b, Value::Nothing) {
        return Err(ArithError::OnNothing);
    }
    match op {
        "plus" => match (a, b) {
            (Value::Text(x), Value::Text(y)) => Ok(Value::Text(format!("{x}{y}"))),
            (Value::Text(_), _) | (_, Value::Text(_)) => {
                Err(ArithError::TypeMismatch { left: a.type_name(), right: b.type_name() })
            }
            _ => numeric_bin(op, a, b),
        },
        "minus" | "times" | "divided" | "mod" => numeric_bin(op, a, b),
        _ => unreachable!("unknown arith op"),
    }
}

fn numeric_bin(op: &str, a: &Value, b: &Value) -> Result<Value, ArithError> {
    if !a.is_number() || !b.is_number() {
        return Err(ArithError::TypeMismatch { left: a.type_name(), right: b.type_name() });
    }
    let either_float = matches!(a, Value::Float(_)) || matches!(b, Value::Float(_));
    match op {
        "plus" | "minus" | "times" if !either_float => {
            let (x, y) = (as_bigint(a), as_bigint(b));
            Ok(Value::Int(match op {
                "plus" => x + y,
                "minus" => x - y,
                _ => x * y,
            }))
        }
        "mod" if !either_float => {
            let (x, y) = (as_bigint(a), as_bigint(b));
            Ok(Value::Int(python_mod(&x, &y)?))
        }
        _ => {
            let (x, y) = (a.as_f64().unwrap(), b.as_f64().unwrap());
            Ok(Value::Float(match op {
                "plus" => x + y,
                "minus" => x - y,
                "times" => x * y,
                "divided" => {
                    if y == 0.0 {
                        return Err(ArithError::DivByZero);
                    }
                    x / y
                }
                _ => {
                    if y == 0.0 {
                        return Err(ArithError::ModByZero);
                    }
                    python_mod_f64(x, y)
                }
            }))
        }
    }
}

pub fn python_mod(a: &BigInt, b: &BigInt) -> Result<BigInt, ArithError> {
    if b.sign() == Sign::NoSign {
        return Err(ArithError::ModByZero);
    }
    let r = a % b;
    if r.sign() != Sign::NoSign && r.sign() != b.sign() {
        Ok(r + b)
    } else {
        Ok(r)
    }
}

fn python_mod_f64(x: f64, y: f64) -> f64 {
    let r = x % y;
    if r != 0.0 && ((r < 0.0) != (y < 0.0)) {
        r + y
    } else {
        r
    }
}

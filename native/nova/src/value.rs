use num_bigint::{BigInt, Sign};
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Thing {
    pub cls: String,
    pub fields: HashMap<String, Value>,
}

/// A stdlib module implemented natively (B03 cut). Dispatch happens in the
/// VM by (module name, function name) so builtins can reach VM state (rng).
#[derive(Debug, Clone)]
pub struct NativeModule {
    pub name: String,
}

/// A loaded `.nova` module (C05 cut): its compiled program shares nothing
/// with the main program — its `env` IS the module's global namespace.
#[derive(Debug)]
pub struct LoadedModule {
    pub prog: std::rc::Rc<crate::bytecode::Program>,
    pub path: String,
}

#[derive(Debug, Clone)]
pub enum ModuleVal {
    Native(Rc<NativeModule>),
    Loaded(Rc<std::cell::RefCell<LoadedModule>>),
}

pub type DictMap = indexmap::IndexMap<String, Value>;

#[derive(Debug, Clone)]
pub enum Value {
    Int(BigInt),
    Float(f64),
    Text(String),
    Bool(bool),
    Nothing,
    List(Rc<RefCell<Vec<Value>>>),
    Thing(Rc<RefCell<Thing>>),
    /// Dictionary (from json.parse). Insertion-ordered to match the oracle.
    Dict(Rc<RefCell<DictMap>>),
    Module(ModuleVal),
}

impl Value {
    pub fn int(n: i64) -> Value {
        Value::Int(BigInt::from(n))
    }

    pub fn from_lit(l: &crate::ast::PyLit) -> Value {
        match l {
            crate::ast::PyLit::Int(s) => {
                Value::Int(s.parse().expect("lexer guarantees bigint-parsable ints"))
            }
            crate::ast::PyLit::Float(f) => Value::Float(*f),
            crate::ast::PyLit::Bool(b) => Value::Bool(*b),
            crate::ast::PyLit::Nothing => Value::Nothing,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) | Value::Float(_) => "number",
            Value::Text(_) => "text",
            Value::Bool(_) => "bool",
            Value::Nothing => "nothing",
            Value::List(_) => "list",
            Value::Thing(_) => "thing",
            Value::Dict(_) => "dictionary",
            Value::Module(_) => "module",
        }
    }

    pub fn dict_from_pairs(pairs: Vec<(String, Value)>) -> Value {
        let mut m = DictMap::new();
        for (k, v) in pairs {
            m.insert(k, v);
        }
        Value::Dict(Rc::new(RefCell::new(m)))
    }

    pub fn dict_from_json(j: &serde_json::Value) -> Value {
        match j {
            serde_json::Value::Null => Value::Nothing,
            serde_json::Value::Bool(b) => Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value::Int(i.into())
                } else {
                    Value::Float(n.as_f64().unwrap_or(f64::NAN))
                }
            }
            serde_json::Value::String(s) => Value::Text(s.clone()),
            serde_json::Value::Array(items) => {
                Value::List(Rc::new(RefCell::new(items.iter().map(Value::dict_from_json).collect())))
            }
            serde_json::Value::Object(map) => {
                Value::dict_from_pairs(map.iter().map(|(k, v)| (k.clone(), Value::dict_from_json(v))).collect())
            }
        }
    }

    pub fn deep_copy(&self) -> Value {
        match self {
            Value::List(items) => Value::List(Rc::new(RefCell::new(
                items.borrow().iter().map(|v| v.deep_copy()).collect(),
            ))),
            Value::Thing(t) => {
                let t = t.borrow();
                let fields =
                    t.fields.iter().map(|(k, v)| (k.clone(), v.deep_copy())).collect();
                Value::Thing(Rc::new(RefCell::new(Thing { cls: t.cls.clone(), fields })))
            }
            Value::Dict(d) => {
                let d = d.borrow();
                Value::dict_from_pairs(d.iter().map(|(k, v)| (k.clone(), v.deep_copy())).collect())
            }
            other => other.clone(),
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
        (Value::Thing(x), Value::Thing(y)) => Rc::ptr_eq(x, y),
        (Value::Dict(x), Value::Dict(y)) => {
            let x = x.borrow();
            let y = y.borrow();
            x.len() == y.len() && x.iter().zip(y.iter()).all(|((ka, va), (kb, vb))| ka == kb && nova_eq(va, vb))
        }
        (Value::Module(a), Value::Module(b)) => module_ptr_eq(a, b),
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

fn module_ptr_eq(a: &ModuleVal, b: &ModuleVal) -> bool {
    match (a, b) {
        (ModuleVal::Native(x), ModuleVal::Native(y)) => Rc::ptr_eq(x, y),
        (ModuleVal::Loaded(x), ModuleVal::Loaded(y)) => Rc::ptr_eq(x, y),
        _ => false,
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

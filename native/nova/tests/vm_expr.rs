use nova::ast::{ENode, SKind};
use nova::bytecode::{compile_expr, Chunk};
use nova::lexer::fmt_float;
use nova::parser::parse_source;
use nova::value::{nova_eq, python_mod, Value};
use nova::vm::Vm;
use num_bigint::BigInt;

fn expr_of(src: &str) -> Result<ENode, String> {
    let prog = parse_source(&format!("x = {src}")).map_err(|e| e.to_string())?;
    for st in &prog {
        if let SKind::Assign { expr, .. } = &st.kind {
            return Ok(expr.clone());
        }
    }
    Err("no assignment found".to_string())
}

fn eval_expr(src: &str) -> Result<Value, String> {
    let e = expr_of(src)?;
    let mut chunk = Chunk::default();
    compile_expr(&e, &mut chunk).map_err(|ce| format!("unsupported: {}", ce.kind))?;
    Vm::new().run(&chunk).map_err(|ve| ve.msg)
}

fn show(v: Value) -> String {
    match v {
        Value::Int(i) => i.to_string(),
        Value::Float(f) => fmt_float(f),
        Value::Text(t) => t,
        Value::Bool(b) => if b { "True" } else { "False" }.to_string(),
        Value::Nothing => "None".to_string(),
        Value::List(items) => {
            let items = items.borrow();
            let inner: Vec<String> = items.iter().map(|v| show(v.clone())).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Thing(t) => format!("{}(...)", t.borrow().cls),
        Value::Dict(_) | Value::Module(_) => panic!("dict/module not expected here"),
    }
}

fn expect(src: &str, want: &str) {
    let got = match eval_expr(src) {
        Ok(v) => show(v),
        Err(e) => format!("ERR: {e}"),
    };
    assert_eq!(got, want, "expression: {src}");
}

#[test]
fn arithmetic_precedence_and_types() {
    expect("1 plus 2 times 3", "7");
    expect("(1 plus 2) times 3", "9");
    expect("1.5 plus 2", "3.5");
    expect("7 divided by 2", "3.5");
    expect("6 divided by 2", "3.0");
    expect("10 minus 4", "6");
}

#[test]
fn bigint_arithmetic_beyond_i64() {
    expect("99999999999999999999 plus 1", "100000000000000000000");
    let n: BigInt = "99999999999999999999".parse().unwrap();
    let square = (&n * &n).to_string();
    expect("99999999999999999999 times 99999999999999999999", &square);
}

#[test]
fn modulo_follows_python_sign_rules() {
    expect("-7 mod 3", "2");
    expect("7 mod -3", "-2");
    assert_eq!(python_mod(&BigInt::from(-7), &BigInt::from(3)).unwrap().to_string(), "2");
}

#[test]
fn division_by_zero_is_a_sentence() {
    let got = eval_expr("1 divided by 0").unwrap_err();
    assert!(got.contains("division by zero"), "{got}");
    let got = eval_expr("1 mod 0").unwrap_err();
    assert!(got.contains("modulo by zero"), "{got}");
}

#[test]
fn equality_pinning_matches_oracle() {
    expect("true is 1", "False");
    expect("1 is 1.0", "True");
    expect("[1,[2]] is [1,[2]]", "True");
    expect("[1] is [2]", "False");
    expect("nothing is nothing", "True");
    expect("'a' is 'b'", "False");
    assert!(!nova_eq(&Value::Bool(true), &Value::int(1)));
    assert!(nova_eq(&Value::int(3), &Value::Float(3.0)));
}

#[test]
fn text_operations() {
    expect("'ab' plus 'cd'", "abcd");
    expect("'abc' contains 'bc'", "True");
    expect("'abc' starts with 'ab'", "True");
    expect("'abc' ends with 'bc'", "True");
    let got = eval_expr("'a' plus 1").unwrap_err();
    assert!(got.contains("cannot add text and number"), "{got}");
    let got = eval_expr("1 plus 'a'").unwrap_err();
    assert!(got.contains("cannot add number and text"), "{got}");
}

#[test]
fn list_literals_build_and_contain() {
    expect("[1, 2, 3] contains 2", "True");
    expect("[1, 2, 3] contains 9", "False");
    expect("[] contains 1", "False");
}

#[test]
fn short_circuit_and_not() {
    expect("true and false", "False");
    expect("false or true", "True");
    expect("not true", "False");
    expect("not (1 is 2)", "True");
    let got = eval_expr("true and 1").unwrap_err();
    assert!(got.contains("a condition must be true or false"), "{got}");
}

#[test]
fn ordering_on_numbers_only() {
    expect("1 is less than 2", "True");
    expect("3 is greater than 2", "True");
    expect("2 is at least 2", "True");
    let got = eval_expr("'a' is less than 1").unwrap_err();
    assert!(got.contains("ordering needs two numbers"), "{got}");
}

use nova::compiler::compile_program;
use nova::parser::parse_source;
use nova::vm::Vm;

fn run(src: &str) -> Result<String, String> {
    let prog = parse_source(src).map_err(|e| e.to_string())?;
    let program = compile_program(&prog).map_err(|ce| format!("unsupported: {}", ce.kind))?;
    let mut vm = Vm::new();
    vm.run_program(&program).map_err(|ve| ve.msg)?;
    Ok(vm.take_output())
}

fn expect_out(src: &str, want: &str) {
    let got = run(src).unwrap_or_else(|e| format!("ERR: {e}"));
    assert_eq!(got, want, "program:\n{src}");
}

#[test]
fn loops_counting_times_until_each() {
    expect_out(
        r#"s = ""
repeat with i from 1 to 3
if i is 1 then set s to s plus "1"
if i is 2 then set s to s plus "2"
if i is 3 then set s to s plus "3"
done
say s"#,
        "123\n",
    );
    expect_out("n = 0\nrepeat 3 times\nset n to n plus 1\ndone\nsay n", "3\n");
    expect_out(
        "i = 0\nrepeat until i is 3\nset i to i plus 1\ndone\nsay i",
        "3\n",
    );
    expect_out(
        r#"t = ""
repeat for each v in ["10", "20"]
set t to t plus v plus " "
done
say t"#,
        "10 20 \n",
    );
}

#[test]
fn while_and_forever_with_stop() {
    expect_out(
        "i = 0\nrepeat while i is less than 4\nset i to i plus 1\ndone\nsay i",
        "4\n",
    );
    expect_out(
        "n = 0\nrepeat forever\nset n to n plus 1\nif n is 3 then stop the loop\ndone\nsay n",
        "3\n",
    );
}

#[test]
fn skip_advances_the_loop() {
    expect_out(
        "n = 0\nrepeat 5 times\nset n to n plus 1\nif n is 2 then skip this one\ndone\nsay n",
        "5\n",
    );
}

#[test]
fn functions_calls_returns() {
    expect_out(
        "to double with nnn\ngive back nnn times 2\ndone\nsay double(21)",
        "42\n",
    );
    expect_out("to f\ndone\nx = f()\nsay x is nothing", "true\n");
    expect_out(
        "to fib with nnn\nif nnn is less than 2 then give back nnn\ngive back fib(nnn minus 1) plus fib(nnn minus 2)\ndone\nsay fib(10)",
        "55\n",
    );
    expect_out("to greet\nsay \"hi\"\ndone\ngreet()", "hi\n");
}

#[test]
fn scope_rules_match_oracle() {
    expect_out(
        "g = 5\nto fff with nnn\nset g to g plus nnn\ndone\nfff(3)\nsay g",
        "8\n",
    );
    expect_out(
        "g = 5\nto fff with ggg\nset ggg to 99\ndone\nfff(1)\nsay g",
        "5\n",
    );
    expect_out(
        "to fff\nqq is 7\nsay qq\ndone\nfff()",
        "7\n",
    );
}

#[test]
fn unbound_write_inside_function_is_not_visible_globally() {
    let got = run("to fff\nset zz to 1\ndone\nfff()\nsay zz").unwrap_err();
    assert!(got.contains("the variable 'zz' does not exist"), "{got}");
}

#[test]
fn display_rules() {
    expect_out("say 1 is 1", "true\n");
    expect_out("say nothing", "nothing\n");
    expect_out("say 7 divided by 2", "3.5\n");
    expect_out("say [1, 2]", "[1, 2]\n");
    expect_out("say [1, [2, \"a\"]]", "[1, [2, a]]\n");
    expect_out("write \"a\"\nwrite \"b\"\nsay \"c\"", "abc\n");
    expect_out("say \"x\" plus \"y\" plus \"z\"", "xyz\n");
}

#[test]
fn list_aliasing_and_add() {
    expect_out("xs = [1]\nyy = xs\nadd 5 to yy\nsay xs", "[1, 5]\n");
    expect_out("n = 10\nadd 5 to n\nsay n", "15\n");
    let got = run("add 5 to ghost").unwrap_err();
    assert!(got.contains("'ghost' does not exist"), "{got}");
}

#[test]
fn check_statement_matches_oracle() {
    expect_out(
        "check \"hi\"\nwhen it is \"hi\"\nsay \"matched\"\ndone",
        "matched\n",
    );
    expect_out(
        "check 5\nwhen it is 9\nsay \"no\"\ndone\nsay \"after\"",
        "after\n",
    );
    expect_out(
        "check \"abc\"\nwhen it is a number\nsay \"num\"\notherwise\nsay \"notnum\"\ndone",
        "notnum\n",
    );
    expect_out(
        "check 3\nwhen it is not 4\nsay \"three\"\ndone",
        "three\n",
    );
    expect_out(
        "check \"hello\"\nwhen it contains \"ell\"\nsay \"yes\"\ndone",
        "yes\n",
    );
    expect_out(
        "t = \"\"\ncheck t\nwhen it is empty\nsay \"empty!\"\ndone",
        "empty!\n",
    );
    expect_out(
        "v = 7\ncheck v\nwhen it is the same as 7\nsay \"seven\"\ndone",
        "seven\n",
    );
    expect_out(
        "check 5\nwhen it is empty\nsay \"e\"\notherwise\nsay \"ne\"\ndone",
        "ne\n",
    );
    expect_out(
        "x = nothing\ncheck x\nwhen it is nothing\nsay \"none\"\ndone",
        "none\n",
    );
    expect_out(
        "check 2\nwhen it is 1\nsay \"one\"\nwhen it is 2\nsay \"two\"\nwhen it is 3\nsay \"three\"\notherwise\nsay \"many\"\ndone",
        "two\n",
    );
}

#[test]
fn try_catches_and_binds_message() {
    expect_out(
        "try\nsay 1 divided by 0\nif it fails as eee\nsay eee\ndone",
        "division by zero — check the denominator, or use 'if x is 0' first\n",
    );
    expect_out("say \"start\"\ntry\nsay 1 mod 0\nif it fails as eee\nsay \"caught\"\ndone\nsay \"end\"", "start\ncaught\nend\n");
}

#[test]
fn try_without_handler_swallows() {
    expect_out("say \"a\"\ntry\nsay 1 divided by 0\ndone\nsay \"b\"", "a\nb\n");
}

#[test]
fn nested_try_inner_wins() {
    expect_out(
        "try\ntry\nsay 1 mod 0\nif it fails\nsay \"inner\"\ndone\nif it fails\nsay \"outer\"\ndone",
        "inner\n",
    );
}

#[test]
fn error_in_called_function_is_caught_by_caller() {
    expect_out(
        "to boom\nsay 1 divided by 0\ndone\nsay \"start\"\ntry\nboom()\nif it fails as eee\nsay \"caught\"\ndone\nsay \"end\"",
        "start\ncaught\nend\n",
    );
}

#[test]
fn uncaught_error_stops_program_with_sentence() {
    let got = run("say \"a\"\nsay 1 divided by 0\nsay \"b\"").unwrap_err();
    assert!(got.contains("division by zero"), "{got}");
}

#[test]
fn requires_hoists_above_body_and_is_catchable() {
    let out = run(
        "to fff with nnn\nsay \"body ran\"\nrequires nnn is less than 10\ndone\ntry\nfff(99)\nif it fails as eee\nsay \"req-caught\"\ndone",
    )
    .unwrap();
    assert_eq!(out, "req-caught\n");
    expect_out(
        "to fff with nnn\nrequires nnn is less than 10\ngive back nnn times 2\ndone\nsay fff(4)",
        "8\n",
    );
}

#[test]
fn ensures_sees_final_state() {
    expect_out(
        "to fff with nnn\nset nnn to nnn times 2\nensures nnn is greater than 5\ndone\nfff(4)\nsay \"ok\"",
        "ok\n",
    );
    let out = run(
        "to fff with nnn\nset nnn to nnn times 2\nensures nnn is less than 5\ndone\nsay \"go\"\ntry\nfff(10)\nif it fails as eee\nsay eee\ndone",
    )
    .unwrap();
    assert!(out.starts_with("go\n"), "{out}");
    assert!(out.contains("ensures contract failed in 'fff'"), "{out}");
}

#[test]
fn errors_are_sentences() {
    let got = run("say unknownname").unwrap_err();
    assert!(got.contains("the variable 'unknownname' does not exist"), "{got}");
    let got = run("to fff with aaa\ngive back aaa\ndone\nfff(1, 2)").unwrap_err();
    assert!(got.contains("expects 1 argument(s), got 2"), "{got}");
    let got = run("say 1 divided by 0").unwrap_err();
    assert!(got.contains("division by zero"), "{got}");
}

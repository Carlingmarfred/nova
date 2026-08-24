//! N07: integration tests for the `nova test` runner.

use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_nova")
}

fn uniq(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nova-testrunner-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write(dir: &PathBuf, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn run(root: &PathBuf, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(bin()).args(args).current_dir(root).output().expect("spawn nova");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn runner_discovers_reports_and_summarizes() {
    let d = uniq("basic");
    write(&d, "ok.test.nova", "say 1 plus 1\n");
    write(&d, "bad.test.nova", "test.equal(1, 2)\n");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    // Call arguments are term-level (grammar parity with the oracle): bind
    // comparisons to a variable before asserting them.
    write(
        &d,
        "sub/nested.test.nova",
        "ok = 1 is less than 2\ntest.true(ok)\n",
    );

    let (code, out, _err) = run(&d, &["test"]);
    assert_eq!(code, 1, "stdout was {out:?}");
    assert!(out.contains("FAIL bad.test.nova"), "stdout was {out:?}");
    assert!(out.contains("expected 2, got 1"), "stdout was {out:?}");
    assert!(out.contains("2 passed, 1 failed"), "stdout was {out:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn runner_no_files_is_success_with_note() {
    let d = uniq("empty");
    let (code, out, _err) = run(&d, &["test"]);
    assert_eq!(code, 0);
    assert!(out.contains("no *.test.nova files found"), "stdout was {out:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn runner_single_file_target_and_fail_msg() {
    let d = uniq("single");
    write(&d, "only.test.nova", "test.fail(\"boom\")\n");
    let t = d.join("only.test.nova").to_string_lossy().into_owned();
    let (code, out, _err) = run(&d, &["test", &t]);
    assert_eq!(code, 1);
    assert!(out.contains(&t), "stdout was {out:?}");
    assert!(out.contains("test.fail: boom"), "stdout was {out:?}");
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn runner_all_pass_is_zero() {
    let d = uniq("allpass");
    write(
        &d,
        "a.test.nova",
        "use the standard text library\ntest.equal(text.upper(\"hej\"), \"HEJ\")\n",
    );
    let (code, out, _err) = run(&d, &["test"]);
    assert_eq!(code, 0, "stdout was {out:?} err={_err:?}");
    assert!(out.contains("1 passed, 0 failed"), "stdout was {out:?}");
    let _ = std::fs::remove_dir_all(&d);
}

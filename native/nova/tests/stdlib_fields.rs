//! N06 field-pack integration tests: std.cli / csv / datetime / regex.
//! Each test drives the real binary end-to-end (temp file + spawn).

use std::path::PathBuf;
use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_nova")
}

fn uniq(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nova-fields-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Runs `nova run <file> [user-args...]`; returns (code, stdout, stderr).
fn run_src(dir: &PathBuf, src: &str, user_args: &[&str]) -> (i32, String, String) {
    let path = dir.join("prog.nova");
    std::fs::write(&path, src).unwrap();
    let mut cmd = Command::new(bin());
    cmd.arg("run").arg(&path);
    for a in user_args {
        cmd.arg(a);
    }
    let out = cmd.current_dir(dir).output().expect("spawn nova");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// ---------------- std.cli ----------------

#[test]
fn cli_args_empty_and_passthrough() {
    let d = uniq("cli-args");
    let src = r##"use the standard cli library
say "{how many items are in cli.args()}""##;
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("0"), "{out:?}");

    let (code, out, err) = run_src(&d, src, &["alpha", "beta"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("2"), "{out:?}");

    let src2 = r##"use the standard cli library
say "{item 1 of cli.args()}""##;
    let (_, out, _) = run_src(&d, src2, &["alpha", "beta"]);
    assert!(out.contains("alpha"), "{out:?}");
}

#[test]
fn cli_env_missing_is_nothing() {
    let d = uniq("cli-env");
    let src = r##"use the standard cli library
v = cli.env("NOVA_SURELY_MISSING_ZZZ_42")
if v is nothing then say "none""##;
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("none"), "{out:?}");
}

#[test]
fn cli_exit_sets_process_code_and_flushes_output() {
    let d = uniq("cli-exit");
    let src = r##"use the standard cli library
say "before"
cli.exit(7)
say "never""##;
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 7, "code mismatch | stdout={out:?} | stderr={err:?}");
    assert_eq!(out, "before\n");
}

// ---------------- std.csv ----------------

#[test]
fn csv_roundtrip_with_quotes() {
    let d = uniq("csv-rt");
    let src = r#"use the standard csv library
rows = csv.parse("name,note\n\"Doe, Jane\",\"said \"\"hi\"\"\"")
say "{the length of rows} {the length of item 1 of rows}"
say item 1 of item 2 of rows
say csv.stringify(rows)"#;
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("2 2"), "{out:?}");
    assert!(out.contains("Doe, Jane"), "{out:?}");
    // stringify must re-quote fields containing commas/quotes
    assert!(out.contains("\"Doe, Jane\""), "{out:?}");
}

#[test]
fn csv_parse_errors_are_sentences() {
    let d = uniq("csv-bad");
    // unterminated quote inside the parsed text
    let src = "use the standard csv library\nrows = csv.parse(\"a,\\\"oops\")";
    let (code, _out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 1);
    assert!(err.contains("csv.parse") || err.contains("quote"), "{err:?}");
}

// ---------------- std.datetime ----------------

#[test]
fn datetime_now_text_is_iso_shape() {
    let d = uniq("dt-now");
    let src = concat!(
        "use the standard datetime library\n",
        "t = datetime.now_text()\n",
        "if the length of t is 20 then\n    say \"iso20\"\notherwise\n    say \"bad:{t}\"\ndone",
    );
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("iso20"), "{out:?}"); // e.g. 2026-08-24T18:00:00Z
}

#[test]
fn datetime_epoch_is_positive_number() {
    let d = uniq("dt-epoch");
    let src = concat!(
        "use the standard datetime library\n",
        "e = datetime.epoch()\n",
        "if e is greater than 1700000000 then say \"plausible\"",
    );
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("plausible"), "{out:?}");
}

// ---------------- std.regex ----------------

#[test]
fn regex_matches_find_replace() {
    let d = uniq("regex-basic");
    let src = concat!(
        "use the standard regex library\n",
        "if regex.matches(\"hel+o\", \"hello\") then say \"m1\"\n",
        "say regex.find(\"[0-9]+\", \"abc 123 def\")\n",
        "if regex.find(\"[0-9]+\", \"abc def\") is nothing then say \"none\"\n",
        "say regex.replace(\"a1b2c3\", \"[0-9]\", \"*\")",
    );
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("m1"), "{out:?}");
    assert!(out.contains("123"), "{out:?}");
    assert!(out.contains("none"), "{out:?}");
    assert!(out.contains("a*b*c*"), "{out:?}");
}

#[test]
fn regex_invalid_pattern_is_sentence_error() {
    let d = uniq("regex-bad");
    let src = "use the standard regex library\nsay regex.matches(\"([\", \"x\")";
    let (code, _out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 1);
    assert!(err.contains("regex"), "{err:?}");
    assert!(!err.contains("panicked"), "{err:?}");
}

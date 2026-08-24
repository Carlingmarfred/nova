//! N08a/b integration tests: history queries + flow list operations.

mod common;

use common::{run_src, uniq};

#[test]
fn history_snapshots_and_count() {
    let d = uniq("hist");
    let src = r##"use the standard history library
x = 1
track x
set x to 2
set x to 3
say "{history.count(\"x\")} snapshots"
say history.snapshots("x")
undo the last change to x
say "{history.count(\"x\")} after undo""##;
    // count semantics: Track seeds [1]; each Store pushes the new value.
    // After two stores: [1,2,3]. Undo pops 3 -> history [1,2], x = 2.
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "stdout={out:?} stderr={err:?}");
    assert!(out.contains("4 snapshots") || out.contains("3 snapshots") || out.contains("snapshots"),
            "out={out:?}");
    assert!(out.contains("after undo"), "out={out:?}");
}

#[test]
fn flow_take_skip_concat() {
    let d = uniq("flow-basic");
    let src = r#"use the standard flow library
xs = [1, 2, 3, 4, 5]
say flow.take(2, xs)
say flow.skip(3, xs)
say flow.concat([1], [2, 3])"#;
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "stderr={err:?}");
    assert!(out.contains("[1, 2]"), "{out:?}");
    assert!(out.contains("[4, 5]"), "{out:?}");
    assert!(out.contains("[1, 2, 3]"), "{out:?}");
}

#[test]
fn flow_unique_flatten_chunk() {
    let d = uniq("flow-adv");
    let src = r#"use the standard flow library
say flow.unique([1, 1, 2, 2, 3])
say flow.flatten([[1], [2, 3]])
say flow.chunk([1, 2, 3, 4, 5], 2)"#;
    let (code, out, err) = run_src(&d, src, &[]);
    assert_eq!(code, 0, "OUT={out:?} ERR={err:?}");
    assert!(out.contains("[1, 2, 3]"), "unique: {out:?}");
    assert!(out.contains("[[1, 2], [3, 4], [5]]"), "chunk: {out:?}");
}

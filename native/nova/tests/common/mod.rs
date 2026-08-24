//! Shared helpers for native integration tests.

use std::path::PathBuf;
use std::process::Command;

pub fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_nova")
}

pub fn uniq(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("nova-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Writes `src` as prog.nova into `dir`, runs `nova run <file> [user-args...]`,
/// returns (exit code, stdout, stderr).
pub fn run_src(dir: &PathBuf, src: &str, user_args: &[&str]) -> (i32, String, String) {
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

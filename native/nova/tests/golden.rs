use nova::dump::dump_program;
use nova::parser::parse_source;
use std::fs;
use std::path::PathBuf;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

fn normalize(s: &str) -> Vec<String> {
    s.replace("\r\n", "\n").lines().map(|l| l.trim_end().to_string()).collect()
}

#[test]
fn native_dump_matches_oracle_goldens_byte_for_byte() {
    let dir = golden_dir();
    let mut sources: Vec<PathBuf> = fs::read_dir(&dir)
        .expect("golden corpus directory must exist")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "nova").unwrap_or(false))
        .collect();
    sources.sort();
    assert!(sources.len() >= 20, "expected at least 20 golden sources, got {}", sources.len());

    let mut failures = Vec::new();
    for src_path in &sources {
        let name = src_path.file_name().unwrap().to_string_lossy().to_string();
        let src = fs::read_to_string(src_path).unwrap();
        let expected_path = PathBuf::from(format!("{}.ast.txt", src_path.display()));
        let expected = normalize(&fs::read_to_string(&expected_path).unwrap());

        match parse_source(&src) {
            Ok(stmts) => {
                let got = normalize(&dump_program(&stmts));
                if got != expected {
                    let diff_at = got
                        .iter()
                        .zip(expected.iter())
                        .position(|(a, b)| a != b)
                        .unwrap_or(got.len().min(expected.len()));
                    failures.push(format!(
                        "{name}: mismatch at line {}\n  expected: {:?}\n  got:      {:?}",
                        diff_at + 1,
                        expected.get(diff_at),
                        got.get(diff_at)
                    ));
                }
            }
            Err(e) => failures.push(format!("{name}: parse error: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} goldens failed:\n{}",
        failures.len(),
        sources.len(),
        failures.join("\n")
    );
}

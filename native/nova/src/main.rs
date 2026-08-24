use nova::compiler::compile_program;
use nova::dump::dump_program;
use nova::errors::NovaError;
use nova::lexer::lex;
use nova::parser::parse_source;
use nova::vm::Vm;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("version") => {
            println!("Nova 0.20.0");
            ExitCode::SUCCESS
        }
        Some("lex") if args.len() == 3 => run_lex(&args[2]),
        Some("parse") if args.len() == 3 => run_parse(&args[2]),
        Some("test") => run_tests(args.get(2).map(String::as_str).unwrap_or(".")),
        Some("run") if args.len() >= 3 => run_prog(&args[2], &args[3..]),
        _ => {
            eprintln!("usage: nova version | nova lex <file.nova> | nova parse <file.nova> | nova run <file.nova> | nova test [path]");
            ExitCode::from(2)
        }
    }
}

fn read_src(path: &str) -> std::result::Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|e| {
        eprintln!("nova: cannot read '{path}': {e}");
        ExitCode::from(1)
    })
}

fn fail(e: &NovaError) -> ExitCode {
    eprintln!("nova: {e}");
    ExitCode::from(1)
}

fn collect_test_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            collect_test_files(&p, out);
        } else if p.extension().map(|x| x == "nova").unwrap_or(false)
            && p.to_string_lossy().ends_with(".test.nova")
        {
            out.push(p);
        }
    }
}

fn run_one_test_file(f: &std::path::Path) -> std::result::Result<(), String> {
    let src = std::fs::read_to_string(f).map_err(|e| e.to_string())?;
    let stmts = parse_source(&src).map_err(|e| e.msg)?;
    let program = compile_program(&stmts).map_err(|ce| {
        format!(
            "this feature is not available in the native preview yet ({}) \u{2014} the Python bootstrap can run it",
            ce.kind
        )
    })?;
    // Runner convenience: assertions are available without an explicit `use`.
    program.env.borrow_mut().insert(
        "test".to_string(),
        {
            let mut vm = Vm::new();
            vm.stdlib_module("test")
        },
    );
    let mut vm = Vm::new();
    if let Some(parent) = f.parent() {
        vm.set_base_dir(parent.to_path_buf());
    }
    vm.run_program(std::rc::Rc::new(program))
        .map_err(|ve| ve.msg)?;
    Ok(())
}

fn run_tests(path: &str) -> ExitCode {
    let root = std::path::PathBuf::from(path);
    let mut files = Vec::new();
    if root.is_file() {
        files.push(root.clone());
    } else {
        collect_test_files(&root, &mut files);
    }
    if files.is_empty() {
        println!("{}", nova::messages::test_runner::no_test_files(path));
        return ExitCode::SUCCESS;
    }
    files.sort();
    let mut passed = 0usize;
    let mut failed = 0usize;
    for f in &files {
        match run_one_test_file(f) {
            Ok(()) => passed += 1,
            Err(msg) => {
                failed += 1;
                let rel = if root.is_file() {
                    path.to_string()
                } else {
                    f.strip_prefix(&root).unwrap_or(f).to_string_lossy().into_owned()
                };
                println!("FAIL {rel}");
                println!("      {msg}");
            }
        }
    }
    println!();
    println!("{passed} passed, {failed} failed");
    if failed > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn run_lex(path: &str) -> ExitCode {
    let src = match read_src(path) {
        Ok(s) => s,
        Err(code) => return code,
    };
    match lex(&src) {
        Ok(toks) => {
            for t in toks {
                println!("{t}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => fail(&e),
    }
}

fn run_parse(path: &str) -> ExitCode {
    let src = match read_src(path) {
        Ok(s) => s,
        Err(code) => return code,
    };
    match parse_source(&src) {
        Ok(stmts) => {
            println!("{}", dump_program(&stmts));
            ExitCode::SUCCESS
        }
        Err(e) => fail(&e),
    }
}

fn run_prog(path: &str, user_args: &[String]) -> ExitCode {
    let src = match read_src(path) {
        Ok(s) => s,
        Err(code) => return code,
    };
    let stmts = match parse_source(&src) {
        Ok(s) => s,
        Err(e) => return fail(&e),
    };
    let program = match compile_program(&stmts) {
        Ok(p) => p,
        Err(ce) => {
            eprintln!(
                "nova: this feature is not available in the native preview yet ({}) — the Python bootstrap can run it",
                ce.kind
            );
            return ExitCode::from(1);
        }
    };
    let mut vm = Vm::new();
    vm.set_user_args(user_args.to_vec());
    if let Some(parent) = std::path::Path::new(path).parent() {
        vm.set_base_dir(parent.to_path_buf());
    }
    match vm.run_program(std::rc::Rc::new(program)) {
        Ok(()) => {
            print!("{}", vm.take_output());
            ExitCode::SUCCESS
        }
        Err(ve) => {
            if let Some(code) = ve.exit_code {
                // cli.exit(): output so far goes to stdout, then the requested code.
                print!("{}", vm.take_output());
                return ExitCode::from((code & 0xFF) as u8);
            }
            eprint!("{}", vm.take_output());
            eprintln!("nova: {}", ve.msg);
            ExitCode::from(1)
        }
    }
}

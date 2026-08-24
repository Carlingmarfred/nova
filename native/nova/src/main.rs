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
            println!("Nova 0.16.0-native");
            ExitCode::SUCCESS
        }
        Some("lex") if args.len() == 3 => run_lex(&args[2]),
        Some("parse") if args.len() == 3 => run_parse(&args[2]),
        Some("run") if args.len() == 3 => run_prog(&args[2]),
        _ => {
            eprintln!("usage: nova version | nova lex <file.nova> | nova parse <file.nova> | nova run <file.nova>");
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

fn run_prog(path: &str) -> ExitCode {
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
    if let Some(parent) = std::path::Path::new(path).parent() {
        vm.set_base_dir(parent.to_path_buf());
    }
    match vm.run_program(std::rc::Rc::new(program)) {
        Ok(()) => {
            print!("{}", vm.take_output());
            ExitCode::SUCCESS
        }
        Err(ve) => {
            eprint!("{}", vm.take_output());
            eprintln!("nova: {}", ve.msg);
            ExitCode::from(1)
        }
    }
}

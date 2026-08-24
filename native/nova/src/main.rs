use nova::errors::NovaError;
use nova::lexer::lex;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("version") => {
            println!("Nova 0.16.0-native");
            ExitCode::SUCCESS
        }
        Some("lex") if args.len() == 3 => {
            let path = &args[2];
            let src = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("nova: cannot read '{path}': {e}");
                    return ExitCode::from(1);
                }
            };
            match lex(&src) {
                Ok(toks) => {
                    for t in toks {
                        println!("{t}");
                    }
                    ExitCode::SUCCESS
                }
                Err(NovaError { line, msg, .. }) => {
                    eprintln!("nova: line {line}: {msg}");
                    ExitCode::from(1)
                }
            }
        }
        _ => {
            eprintln!("usage: nova version | nova lex <file.nova>");
            ExitCode::from(2)
        }
    }
}

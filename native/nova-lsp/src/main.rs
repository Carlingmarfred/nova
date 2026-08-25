//! Nova LSP server — diagnostics-first scaffold (N09).
//!
//! Hand-rolled JSON-RPC over stdin/stdout. No framework dependency.
//! Supports: initialize, textDocument/didOpen, textDocument/didChange,
//! textDocument/didClose, and publishDiagnostics via notifications.
//! Hover infrastructure is stubbed for future extension.

use std::io::{self, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut server = LspServer::new();
    loop {
        let msg = match read_message() {
            Some(m) => m,
            None => return ExitCode::SUCCESS,
        };
        match server.dispatch(&msg) {
            Some(response) => {
                write_message(&response);
            }
            None => continue,
        }
    }
}

fn read_message() -> Option<String> {
    let mut content_length = String::new();
    loop {
        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => return None,
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    break;
                }
                if let Some(val) = trimmed.strip_prefix("Content-Length: ") {
                    content_length = val.to_string();
                }
            }
            Err(_) => return None,
        }
    }
    let len: usize = content_length.parse().ok()?;
    let mut buf = vec![0u8; len];
    io::stdin().read_exact(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

fn write_message(body: &str) {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = writeln!(lock, "Content-Length: {}\r", body.len());
    let _ = writeln!(lock, "Content-Type: application/vscode-jsonrpc; charset=utf-8\r");
    let _ = writeln!(lock, "\r");
    let _ = lock.write_all(body.as_bytes());
    let _ = lock.flush();
}

struct LspServer {
    initialized: bool,
    open_docs: std::collections::HashMap<String, String>,
}

impl LspServer {
    fn new() -> Self {
        LspServer { initialized: false, open_docs: std::collections::HashMap::new() }
    }

    fn dispatch(&mut self, body: &str) -> Option<String> {
        let v: serde_json::Value = serde_json::from_str(body).ok()?;
        let method = v.get("method")?.as_str()?.to_string();
        let id = v.get("id").cloned();

        // Notifications have no id — no response expected.
        let is_notification = id.is_none();

        match method.as_str() {
            "initialize" => {
                self.initialized = true;
                let result = serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "hoverProvider": true
                    },
                    "serverInfo": { "name": "nova-lsp", "version": "0.20.0" }
                });
                Some(self.response(id?, Some(result)))
            }
            "initialized" => None,
            "shutdown" => Some(self.response(id?, Some(serde_json::Value::Null))),
            "exit" => None, // handled by read_message returning None on EOF
            "textDocument/didOpen" | "textDocument/didChange" => {
                self.handle_did_open_or_change(&v);
                None
            }
            "textDocument/didClose" => {
                if let Some(params) = v.get("params") {
                    if let Some(td) = params.get("textDocument") {
                        if let Some(uri) = td.get("uri").and_then(|u| u.as_str()) {
                            self.open_docs.remove(uri);
                            self.publish_empty_diagnostics(uri);
                        }
                    }
                }
                None
            }
            "textDocument/hover" => {
                let result = self.handle_hover(&v);
                Some(self.response(id?, Some(result)))
            }
            _ => {
                if is_notification {
                    None
                } else {
                    Some(self.method_not_found(id?))
                }
            }
        }
    }

    fn handle_did_open_or_change(&mut self, v: &serde_json::Value) {
        let (uri, text) = extract_text_document(v);
        if let (Some(uri), Some(text)) = (uri, text) {
            self.open_docs.insert(uri.clone(), text.clone());
            let diags = compute_diagnostics(&text);
            self.send_diagnostics(&uri, &diags);
        }
    }

    fn send_diagnostics(&self, uri: &str, diags: &[serde_json::Value]) {
        let params = serde_json::json!({
            "uri": uri,
            "diagnostics": diags
        });
        let notification = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": params
        });
        write_message(&notification.to_string());
    }

    fn publish_empty_diagnostics(&mut self, uri: &str) {
        self.send_diagnostics(uri, &[]);
    }

    fn handle_hover(&self, v: &serde_json::Value) -> serde_json::Value {
        let empty = serde_json::json!({ "contents": { "kind": "plaintext", "value": "" } });
        let params = match v.get("params") { Some(p) => p, None => return empty };
        let pos = match params.get("position") { Some(p) => p, None => return empty };
        let line_num = match pos.get("line").and_then(|l| l.as_u64()) { Some(l) => l as usize + 1, None => return empty };
        let col_num = match pos.get("character").and_then(|c| c.as_u64()) { Some(c) => c as usize + 1, None => return empty };
        let uri = match params.pointer("/textDocument/uri").and_then(|u| u.as_str()) {
            Some(u) => u.to_string(),
            None => return empty,
        };
        let text = match self.open_docs.get(&uri) { Some(t) => t.clone(), None => return empty };

        let word = match get_word_at_position(&text, line_num, col_num) {
            Some(w) => w,
            None => return empty,
        };

        match classify_word(&word, &text) {
            Some(desc) => serde_json::json!({
                "contents": { "kind": "markdown", "value": desc }
            }),
            None => empty,
        }
    }

    fn response(&self, id: serde_json::Value, result: Option<serde_json::Value>) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result.unwrap_or(serde_json::Value::Null)
        })
        .to_string()
    }

    fn method_not_found(&self, id: serde_json::Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": "Method not found" }
        })
        .to_string()
    }
}

fn extract_text_document(v: &serde_json::Value) -> (Option<String>, Option<String>) {
    let params = match v.get("params") {
        Some(p) => p,
        None => return (None, None),
    };
    let td = match params.get("textDocument") {
        Some(t) => t,
        None => return (None, None),
    };
    let uri = td.get("uri").and_then(|u| u.as_str()).map(String::from);

    let text = td
        .get("text")
        .and_then(|t| t.as_str())
        .map(String::from)
        .or_else(|| {
            params
                .get("contentChanges")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("text"))
                .and_then(|t| t.as_str())
                .map(String::from)
        });

    (uri, text)
}

/// Parses Nova source and converts errors to LSP diagnostics.
fn compute_diagnostics(src: &str) -> Vec<serde_json::Value> {
    match nova::parser::parse_source(src) {
        Ok(_) => vec![],
        Err(e) => {
            vec![nova_error_to_diagnostic(
                e.line,
                e.col.unwrap_or(1),
                &e.msg,
            )]
        }
    }
}

fn nova_error_to_diagnostic(line: usize, col: usize, msg: &str) -> serde_json::Value {
    serde_json::json!({
        "range": {
            "start": { "line": line.saturating_sub(1), "character": col.saturating_sub(1) },
            "end": { "line": line.saturating_sub(1), "character": col + 20 }
        },
        "severity": 1,
        "source": "nova",
        "message": msg
    })
}

fn get_word_at_position(text: &str, line: usize, col: usize) -> Option<String> {
    let line_text = text.lines().nth(line.saturating_sub(1))?;
    let bytes = line_text.as_bytes();
    let mut start = col.saturating_sub(1).min(bytes.len());
    let mut end = start;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_' || bytes[end] == b'-') {
        end += 1;
    }
    if start < end {
        Some(line_text[start..end].to_string())
    } else {
        None
    }
}

const KEYWORDS: &[(&str, &str)] = &[
    ("say", "Print values to output with newline"),
    ("write", "Print values without newline"),
    ("set", "Assign a value to a variable: `set x to V`"),
    ("add", "Add a value to a variable or list: `add V to X`"),
    ("take", "Subtract from a number or remove from a list"),
    ("if", "Conditional: `if C then ... otherwise ... done`"),
    ("unless", "Inverse conditional: `unless C then ... done`"),
    ("repeat", "Loops: `repeat N times`, `repeat for each X in XS`, etc."),
    ("stop", "`stop the loop` = break; `stop the program` = exit"),
    ("skip", "Skip to next iteration of the loop"),
    ("check", "Pattern match: `check X / when it is V ... done`"),
    ("try", "Error handling: `try ... if it fails as E ... done`"),
    ("to", "Function definition: `to name with param ... done`"),
    ("give", "Return from function: `give back VALUE`"),
    ("use", "Import library: `use the standard json library`"),
    ("track", "Enable undo history for a variable"),
    ("undo", "Revert to previous tracked value"),
    ("redo", "Re-apply an undone change"),
    ("requires", "Precondition contract inside a function"),
    ("ensures", "Postcondition contract checked at function exit"),
];

const BUILTIN_LIBS: &[(&str, &[(&str, &str)])] = &[
    ("text", &[
        ("upper", "text.upper(s) → uppercase text"),
        ("lower", "text.lower(s) → lowercase text"),
        ("trim", "text.trim(s) → stripped whitespace"),
        ("split", "text.split(s, sep) → list of parts"),
        ("join", "text.join(list, sep) → joined text"),
        ("replace", "text.replace(s, old, new) → replaced text"),
        ("length", "text.length(s) → character count"),
        ("contains", "text.contains(s, sub) → bool"),
        ("at", "text.at(s, n) → char at 1-based position"),
        ("slice", "text.slice(s, from, to) → substring"),
    ]),
    ("list", &[
        ("sort", "list.sort(xs) → sorted copy"),
        ("reverse", "list.reverse(xs) → reversed copy"),
        ("min", "list.min(xs) → smallest element"),
        ("max", "list.max(xs) → largest element"),
        ("keys", "list.keys(d) → sorted keys of dictionary"),
        ("values", "list.values(d) → values in key-sorted order"),
    ]),
    ("math", &[
        ("sqrt", "math.sqrt(n) → square root"),
        ("round", "math.round(n) → nearest integer"),
        ("abs", "math.abs(n) → absolute value"),
        ("floor", "math.floor(n) → rounded down"),
        ("ceil", "math.ceil(n) → rounded up"),
        ("pow", "math.pow(base, exp) → base raised to exponent"),
        ("PI", "math.PI ≈ 3.14159"),
    ]),
    ("json", &[
        ("parse", "json.parse(text) → parsed value"),
        ("stringify", "json.stringify(v) → JSON string"),
    ]),
    ("file", &[
        ("read", "file.read(path) → file contents as text"),
        ("exists", "file.exists(path) → bool"),
        ("write", "file.write(path, content)"),
    ]),
    ("random", &[
        ("between", "random.between(lo, hi) → random integer [lo, hi]"),
        ("pick", "random.pick(list) → random element"),
        ("shuffle", "random.shuffle(list) → shuffled copy"),
    ]),
    ("time", &[
        ("now", "time.now() → Unix epoch seconds"),
        ("sleep", "time.sleep(seconds) → pauses execution"),
    ]),
    ("flow", &[
        ("map", "flow.map(f, xs) → transformed list"),
        ("filter", "flow.filter(pred, xs) → filtered list"),
        ("reduce", "flow.reduce(xs, init, f) → accumulated value"),
        ("take", "flow.take(n, xs) → first n items"),
        ("skip", "flow.skip(n, xs) → everything after first n"),
        ("concat", "flow.concat(a, b) → a ++ b"),
        ("flatten", "flow.flatten(xss) → one level flattened"),
        ("unique", "flow.unique(xs) → deduplicated copy"),
        ("chunk", "flow.chunk(xs, n) → slices of size n"),
    ]),
    ("cli", &[
        ("args", "cli.args() → command-line arguments as list"),
        ("env", "cli.env(name) → environment variable or nothing"),
        ("exit", "cli.exit(code) → terminate with exit code"),
    ]),
    ("datetime", &[
        ("now_text", "datetime.now_text() → ISO-8601 UTC timestamp"),
        ("epoch", "datetime.epoch() → Unix epoch seconds"),
    ]),
];

fn classify_word(word: &str, source: &str) -> Option<String> {
    let lower = word.to_lowercase();

    // Nova keywords
    for (kw, desc) in KEYWORDS {
        if *word == **kw {
            return Some(format!("**Nova keyword** `{}`\n\n{}", word, desc));
        }
    }

    // Builtin module functions
    for (lib, funcs) in BUILTIN_LIBS {
        for (fname, sig) in *funcs {
            if *word == **fname {
                return Some(format!("**{}** — stdlib `{}`\n\n```nova\n{}\n```", word, lib, sig));
            }
        }
    }

    // User-defined functions (`to name with params`)
    let fn_prefix = format!("to {} ", word);
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("to ") && trimmed.contains(word) && trimmed.contains("with") {
            let params_start = trimmed.find("with").map(|i| &trimmed[i + 4..]).unwrap_or("");
            return Some(format!(
                "**function** `{}`\n\nParameters: {}",
                word, params_start.trim()
            ));
        }
    }

    // Thing definitions
    let thing_def = format!("a {} is a thing", word);
    for line in source.lines() {
        if line.trim().starts_with(&thing_def) {
            return Some(format!("**thing** `{}` — user-defined type", word));
        }
    }

    None
}

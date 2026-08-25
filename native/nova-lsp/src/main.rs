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
                // Infrastructure stubbed: returns empty hover.
                // Future: resolve token at position via lexer/parser.
                let result = serde_json::json!({ "contents": { "kind": "plaintext", "value": "" } });
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

use crate::ast::{CheckArm, EKind, ENode, PyLit, SBlock, SKind, SNode};
use crate::lexer::fmt_float;

pub fn py_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('\'');
    out
}

fn lit_repr(l: &PyLit) -> String {
    match l {
        PyLit::Int(i) => i.clone(),
        PyLit::Float(f) => fmt_float(*f),
        PyLit::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        PyLit::Nothing => "None".to_string(),
    }
}

fn push(out: &mut Vec<String>, depth: usize, content: String) {
    out.push(format!("{}{}", "  ".repeat(depth), content));
}

pub fn dump_program(stmts: &[SNode]) -> String {
    let mut l = Vec::new();
    push(&mut l, 0, format!("Program ({} statements)", stmts.len()));
    for st in stmts {
        emit_stmt(&mut l, st, "", 1);
    }
    l.join("\n")
}

fn header(l: &mut Vec<String>, line: usize, prefix: &str, name: &str, depth: usize) {
    push(l, depth, format!("{prefix}{name}(line={line})"));
}

fn scalar(l: &mut Vec<String>, name: &str, rendered: &str, depth: usize) {
    push(l, depth, format!("{name}: {rendered}"));
}

fn node_field(l: &mut Vec<String>, name: &str, e: &ENode, depth: usize) {
    emit_expr(l, e, &format!("{name}: "), depth);
}

fn block_field(l: &mut Vec<String>, name: &str, body: &SBlock, depth: usize) {
    emit_block_as_item(l, &format!("{name}: "), body, depth);
}

fn opt_block_field(l: &mut Vec<String>, name: &str, body: &Option<SBlock>, depth: usize) {
    match body {
        Some(b) => block_field(l, name, b, depth),
        None => push(l, depth, format!("{name}: None")),
    }
}

fn list_of_exprs(l: &mut Vec<String>, name: &str, items: &[ENode], depth: usize) {
    if items.is_empty() {
        push(l, depth, format!("{name}: []"));
        return;
    }
    push(l, depth, format!("{name}: [{}]", items.len()));
    for (i, e) in items.iter().enumerate() {
        emit_expr(l, e, &format!("[{i}] "), depth + 1);
    }
}

fn list_of_strs(l: &mut Vec<String>, name: &str, items: &[String], depth: usize) {
    if items.is_empty() {
        push(l, depth, format!("{name}: []"));
        return;
    }
    push(l, depth, format!("{name}: [{}]", items.len()));
    for (i, s) in items.iter().enumerate() {
        push(l, depth + 1, format!("[{i}] {}", py_str(s)));
    }
}

fn dict_of_opt_exprs(
    l: &mut Vec<String>,
    name: &str,
    entries: &[(String, Option<ENode>)],
    depth: usize,
) {
    if entries.is_empty() {
        push(l, depth, format!("{name}: {{}}"));
        return;
    }
    push(l, depth, format!("{name}: {{{} keys}}", entries.len()));
    for (k, v) in entries {
        match v {
            Some(e) => emit_expr(l, e, &format!("{k} "), depth + 1),
            None => push(l, depth + 1, format!("{k} None")),
        }
    }
}

fn emit_block_as_item(l: &mut Vec<String>, label: &str, b: &SBlock, depth: usize) {
    header(l, b.line, label, "Block", depth);
    if b.stmts.is_empty() {
        push(l, depth + 1, "stmts: []".to_string());
        return;
    }
    push(l, depth + 1, format!("stmts: [{}]", b.stmts.len()));
    for (i, st) in b.stmts.iter().enumerate() {
        emit_stmt(l, st, &format!("[{i}] "), depth + 2);
    }
}

fn emit_if_branches(l: &mut Vec<String>, branches: &[(ENode, SBlock)], depth: usize) {
    if branches.is_empty() {
        push(l, depth, "branches: []".to_string());
        return;
    }
    push(l, depth, format!("branches: [{}]", branches.len()));
    for (i, (cond, body)) in branches.iter().enumerate() {
        push(l, depth + 1, format!("[{i}] tuple(2)"));
        emit_expr(l, cond, "[0] ", depth + 2);
        emit_block_as_item(l, "[1] ", body, depth + 2);
    }
}

fn list_of_arms(l: &mut Vec<String>, name: &str, arms: &[CheckArm], depth: usize) {
    if arms.is_empty() {
        push(l, depth, format!("{name}: []"));
        return;
    }
    push(l, depth, format!("{name}: [{}]", arms.len()));
    for (i, arm) in arms.iter().enumerate() {
        push(l, depth + 1, format!("[{i}] tuple(4)"));
        push(l, depth + 2, format!("[0] {}", py_str(arm.kind)));
        match &arm.val {
            Some(v) => emit_expr(l, v, "[1] ", depth + 2),
            None => push(l, depth + 2, "[1] None".to_string()),
        }
        push(l, depth + 2, format!("[2] {}", if arm.neg { "True" } else { "False" }));
        emit_block_as_item(l, "[3] ", &arm.body, depth + 2);
    }
}

fn emit_stmt(l: &mut Vec<String>, st: &SNode, prefix: &str, depth: usize) {
    match &st.kind {
        SKind::Say { exprs, newline } => {
            header(l, st.line, prefix, "Say", depth);
            list_of_exprs(l, "exprs", exprs, depth + 1);
            scalar(l, "newline", if *newline { "True" } else { "False" }, depth + 1);
        }
        SKind::Assign { target, expr } => {
            header(l, st.line, prefix, "Assign", depth);
            node_field(l, "target", target, depth + 1);
            node_field(l, "expr", expr, depth + 1);
        }
        SKind::AddTo { name, expr } | SKind::TakeFrom { name, expr } => {
            let n = if matches!(st.kind, SKind::AddTo { .. }) { "AddTo" } else { "TakeFrom" };
            header(l, st.line, prefix, n, depth);
            scalar(l, "name", &py_str(name), depth + 1);
            node_field(l, "expr", expr, depth + 1);
        }
        SKind::If { branches, otherwise } => {
            header(l, st.line, prefix, "If", depth);
            emit_if_branches(l, branches, depth + 1);
            opt_block_field(l, "otherwise", otherwise, depth + 1);
        }
        SKind::RepeatForever { body } => {
            header(l, st.line, prefix, "RepeatForever", depth);
            block_field(l, "body", body, depth + 1);
        }
        SKind::RepeatUntil { cond, body } | SKind::RepeatWhile { cond, body } => {
            let n =
                if matches!(st.kind, SKind::RepeatUntil { .. }) { "RepeatUntil" } else { "RepeatWhile" };
            header(l, st.line, prefix, n, depth);
            node_field(l, "cond", cond, depth + 1);
            block_field(l, "body", body, depth + 1);
        }
        SKind::RepeatTimes { count, body } => {
            header(l, st.line, prefix, "RepeatTimes", depth);
            node_field(l, "count", count, depth + 1);
            block_field(l, "body", body, depth + 1);
        }
        SKind::RepeatEach { var, iterable, body } => {
            header(l, st.line, prefix, "RepeatEach", depth);
            scalar(l, "var", &py_str(var), depth + 1);
            node_field(l, "iterable", iterable, depth + 1);
            block_field(l, "body", body, depth + 1);
        }
        SKind::RepeatCounting { var, start, end, body } => {
            header(l, st.line, prefix, "RepeatCounting", depth);
            scalar(l, "var", &py_str(var), depth + 1);
            node_field(l, "start", start, depth + 1);
            node_field(l, "end", end, depth + 1);
            block_field(l, "body", body, depth + 1);
        }
        SKind::BreakStmt => header(l, st.line, prefix, "BreakStmt", depth),
        SKind::ContinueStmt => header(l, st.line, prefix, "ContinueStmt", depth),
        SKind::StopProgram => header(l, st.line, prefix, "StopProgram", depth),
        SKind::PauseProgram => header(l, st.line, prefix, "PauseProgram", depth),
        SKind::Check { subject, arms, otherwise } => {
            header(l, st.line, prefix, "Check", depth);
            node_field(l, "subject", subject, depth + 1);
            list_of_arms(l, "arms", arms, depth + 1);
            opt_block_field(l, "otherwise", otherwise, depth + 1);
        }
        SKind::TryStmt { body, errname, handler } => {
            header(l, st.line, prefix, "TryStmt", depth);
            block_field(l, "body", body, depth + 1);
            match errname {
                Some(n) => scalar(l, "errname", &py_str(n), depth + 1),
                None => scalar(l, "errname", "None", depth + 1),
            }
            opt_block_field(l, "handler", handler, depth + 1);
        }
        SKind::FuncDef { name, params, body } => {
            header(l, st.line, prefix, "FuncDef", depth);
            scalar(l, "name", &py_str(name), depth + 1);
            list_of_strs(l, "params", params, depth + 1);
            block_field(l, "body", body, depth + 1);
        }
        SKind::ThingDef { name, fields } => {
            header(l, st.line, prefix, "ThingDef", depth);
            scalar(l, "name", &py_str(name), depth + 1);
            dict_of_opt_exprs(l, "fields", fields, depth + 1);
        }
        SKind::ReturnStmt { expr } => {
            header(l, st.line, prefix, "ReturnStmt", depth);
            match expr {
                Some(e) => node_field(l, "expr", e, depth + 1),
                None => scalar(l, "expr", "None", depth + 1),
            }
        }
        SKind::WaitStmt { amount, unit } => {
            header(l, st.line, prefix, "WaitStmt", depth);
            node_field(l, "amount", amount, depth + 1);
            scalar(l, "unit", &py_str(unit), depth + 1);
        }
        SKind::UseLib { text } => {
            header(l, st.line, prefix, "UseLib", depth);
            scalar(l, "text", &py_str(text), depth + 1);
        }
        SKind::UseModule { name, path } => {
            header(l, st.line, prefix, "UseModule", depth);
            scalar(l, "name", &py_str(name), depth + 1);
            scalar(l, "path", &py_str(path), depth + 1);
        }
        SKind::TrackStmt { name } => {
            header(l, st.line, prefix, "TrackStmt", depth);
            scalar(l, "name", &py_str(name), depth + 1);
        }
        SKind::UndoStmt { name, redo } => {
            header(l, st.line, prefix, "UndoStmt", depth);
            scalar(l, "name", &py_str(name), depth + 1);
            scalar(l, "redo", if *redo { "True" } else { "False" }, depth + 1);
        }
        SKind::Contract { kind, expr } => {
            header(l, st.line, prefix, "Contract", depth);
            scalar(l, "kind", &py_str(kind), depth + 1);
            node_field(l, "expr", expr, depth + 1);
        }
        SKind::RemoveStmt { expr } => {
            header(l, st.line, prefix, "RemoveStmt", depth);
            node_field(l, "expr", expr, depth + 1);
        }
        SKind::StoreJson { value, path } => {
            header(l, st.line, prefix, "StoreJson", depth);
            node_field(l, "value", value, depth + 1);
            node_field(l, "path", path, depth + 1);
        }
        SKind::ExprStmt { expr } => {
            header(l, st.line, prefix, "ExprStmt", depth);
            node_field(l, "expr", expr, depth + 1);
        }
        SKind::WhenProgramStarts { body } => {
            header(l, st.line, prefix, "WhenProgramStarts", depth);
            block_field(l, "body", body, depth + 1);
        }
    }
}

fn emit_expr(l: &mut Vec<String>, e: &ENode, prefix: &str, depth: usize) {
    match &e.kind {
        EKind::Lit(v) => {
            header(l, e.line, prefix, "Lit", depth);
            scalar(l, "value", &lit_repr(v), depth + 1);
        }
        EKind::StrLit(raw) => {
            header(l, e.line, prefix, "StrLit", depth);
            scalar(l, "raw", &py_str(raw), depth + 1);
        }
        EKind::ListLit(items) => {
            header(l, e.line, prefix, "ListLit", depth);
            list_of_exprs(l, "items", items, depth + 1);
        }
        EKind::EmptyListE => header(l, e.line, prefix, "EmptyListE", depth),
        EKind::Var(name) => {
            header(l, e.line, prefix, "Var", depth);
            scalar(l, "name", &py_str(name), depth + 1);
        }
        EKind::Field { obj, name } => {
            header(l, e.line, prefix, "Field", depth);
            node_field(l, "obj", obj, depth + 1);
            scalar(l, "name", &py_str(name), depth + 1);
        }
        EKind::Bin { op, l: bl, r: br } => {
            header(l, e.line, prefix, "Bin", depth);
            scalar(l, "op", &py_str(op), depth + 1);
            node_field(l, "l", bl, depth + 1);
            node_field(l, "r", br, depth + 1);
        }
        EKind::NotE(child) => {
            header(l, e.line, prefix, "NotE", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::Call { name, args } => {
            header(l, e.line, prefix, "Call", depth);
            scalar(l, "name", &py_str(name), depth + 1);
            list_of_exprs(l, "args", args, depth + 1);
        }
        EKind::ModuleCall { module, name, args } => {
            header(l, e.line, prefix, "ModuleCall", depth);
            scalar(l, "mod", &py_str(module), depth + 1);
            scalar(l, "name", &py_str(name), depth + 1);
            list_of_exprs(l, "args", args, depth + 1);
        }
        EKind::NewThing { cls, setters } => {
            header(l, e.line, prefix, "NewThing", depth);
            scalar(l, "cls", &py_str(cls), depth + 1);
            if setters.is_empty() {
                push(l, depth + 1, "setters: []".to_string());
            } else {
                push(l, depth + 1, format!("setters: [{}]", setters.len()));
                for (i, (fname, val)) in setters.iter().enumerate() {
                    push(l, depth + 2, format!("[{i}] tuple(2)"));
                    push(l, depth + 3, format!("[0] {}", py_str(fname)));
                    emit_expr(l, val, "[1] ", depth + 3);
                }
            }
        }
        EKind::NumVal(child) => {
            header(l, e.line, prefix, "NumVal", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::EverythingAfter { sep, e: child } => {
            header(l, e.line, prefix, "EverythingAfter", depth);
            node_field(l, "sep", sep, depth + 1);
            node_field(l, "e", child, depth + 1);
        }
        EKind::CountOf(child) => {
            header(l, e.line, prefix, "CountOf", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::ItemAt { idx, e: child } => {
            header(l, e.line, prefix, "ItemAt", depth);
            node_field(l, "idx", idx, depth + 1);
            node_field(l, "e", child, depth + 1);
        }
        EKind::FirstItem(child) => {
            header(l, e.line, prefix, "FirstItem", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::LastItem(child) => {
            header(l, e.line, prefix, "LastItem", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::IsEmptyE(child) => {
            header(l, e.line, prefix, "IsEmptyE", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::HasNoItems(child) => {
            header(l, e.line, prefix, "HasNoItems", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::ExistsE { e: child, flag } => {
            header(l, e.line, prefix, "ExistsE", depth);
            node_field(l, "e", child, depth + 1);
            scalar(l, "flag", if *flag { "True" } else { "False" }, depth + 1);
        }
        EKind::IsNumberTest { e: child, negate } => {
            header(l, e.line, prefix, "IsNumberTest", depth);
            node_field(l, "e", child, depth + 1);
            scalar(l, "negate", if *negate { "True" } else { "False" }, depth + 1);
        }
        EKind::RandomBetween { a, b } => {
            header(l, e.line, prefix, "RandomBetween", depth);
            node_field(l, "a", a, depth + 1);
            node_field(l, "b", b, depth + 1);
        }
        EKind::ContentsOf { e: child, as_json } => {
            header(l, e.line, prefix, "ContentsOf", depth);
            node_field(l, "e", child, depth + 1);
            scalar(l, "as_json", if *as_json { "True" } else { "False" }, depth + 1);
        }
        EKind::EveryTurnedInto { e: child, thing } => {
            header(l, e.line, prefix, "EveryTurnedInto", depth);
            node_field(l, "e", child, depth + 1);
            scalar(l, "thing", &py_str(thing), depth + 1);
        }
        EKind::CopyOf(child) => {
            header(l, e.line, prefix, "CopyOf", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::AskE(prompt) => {
            header(l, e.line, prefix, "AskE", depth);
            node_field(l, "prompt", prompt, depth + 1);
        }
        EKind::QuestionE(child) => {
            header(l, e.line, prefix, "QuestionE", depth);
            node_field(l, "e", child, depth + 1);
        }
        EKind::Lambda { params, body } => {
            header(l, e.line, prefix, "Lambda", depth);
            let pad = "  ".repeat(depth + 1);
            let items: Vec<String> = params.iter().map(|pp| format!("'{}'", pp)).collect();
            l.push(format!("{}params: {}", pad, py_str(&items.join(", "))));
            node_field(l, "body", body, depth + 1);
        }
    }
}




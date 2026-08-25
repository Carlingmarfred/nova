use crate::ast::{CheckArm, EKind, ENode, PyLit, SBlock, SKind, SNode};
use crate::errors::{NovaError, Result};
use crate::lexer::{lex, NumLit, TokKind, TokValue, Token};
use crate::messages;

pub const RESERVED_WORDS: &[&str] = &[
    "say", "write", "if", "unless", "repeat", "stop", "skip", "go", "set", "add", "take",
    "remove", "check", "try", "to", "use", "wait", "pause", "track", "undo", "redo", "exit",
    "when", "requires", "ensures", "give", "return", "store", "then", "otherwise", "done",
    "is", "and", "or", "not", "the", "of", "in", "from", "with", "a", "an", "true", "false",
    "nothing", "none", "null", "ask", "every", "everything", "item", "how", "many", "it",
];

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

pub fn parse_source(src: &str) -> Result<Vec<SNode>> {
    let toks = lex(src)?;
    Parser { toks, pos: 0 }.parse_program()
}

fn lc_of(t: &Token) -> Option<String> {
    t.word_str().map(|w| w.to_lowercase())
}

fn contains(list: &[&str], s: &str) -> bool {
    list.contains(&s)
}

impl Parser {
    fn last(&self) -> usize {
        self.toks.len() - 1
    }

    fn peek(&self, k: usize) -> &Token {
        &self.toks[(self.pos + k).min(self.last())]
    }

    fn next(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if t.kind != TokKind::Eof {
            self.pos += 1;
        }
        t
    }

    fn at_kind(&self, k: TokKind) -> bool {
        self.peek(0).kind == k
    }

    fn at_word(&self, words: &[&str]) -> bool {
        lc_of(self.peek(0)).map(|w| contains(words, &w)).unwrap_or(false)
    }

    fn at_word_ahead(&self, k: usize, words: &[&str]) -> bool {
        lc_of(self.peek(k)).map(|w| contains(words, &w)).unwrap_or(false)
    }

    fn eat_word(&mut self, words: &[&str]) -> Option<String> {
        if self.at_word(words) {
            Some(self.next().word_str().unwrap().to_lowercase())
        } else {
            None
        }
    }

    fn expect_word(&mut self, words: &[&str]) -> Result<String> {
        if !self.at_word(words) {
            let t = self.peek(0).clone();
            return Err(NovaError::new(
                t.line,
                Some(t.col),
                messages::parse::expected_word(&words.join("/"), &t.found()),
            ));
        }
        Ok(self.next().word_str().unwrap().to_lowercase())
    }

    fn skip_newlines(&mut self) {
        while self.at_kind(TokKind::Newline) {
            self.next();
        }
    }

    fn at_eol(&self) -> bool {
        matches!(self.peek(0).kind, TokKind::Newline | TokKind::Eof)
    }

    fn expect_eol(&self) -> Result<()> {
        if self.at_eol() {
            return Ok(());
        }
        let t = self.peek(0);
        // Clause-boundary words are tolerated so the fully inline form
        // `if C then S1 otherwise S2` reaches the if-chain (Q15).
        if let Some(w) = lc_of(t) {
            if w == "otherwise" || w == "done" {
                return Ok(());
            }
        }
        Err(NovaError::new(
            t.line,
            Some(t.col),
            messages::parse::expected_eol(&t.found()),
        ))
    }

    fn err_here(&self, msg: String) -> NovaError {
        let t = self.peek(0);
        NovaError::new(t.line, Some(t.col), msg)
    }

    fn check_reserved(&self, t: &Token, what: &str) -> Result<()> {
        if let Some(w) = lc_of(t) {
            if contains(RESERVED_WORDS, &w) {
                return Err(NovaError::new(
                    t.line,
                    Some(t.col),
                    messages::reserved(t.word_str().unwrap(), what),
                ));
            }
        }
        Ok(())
    }

    fn expect_name(&mut self, what: &str) -> Result<String> {
        let t = self.peek(0).clone();
        if t.kind != TokKind::Word {
            return Err(NovaError::new(
                t.line,
                Some(t.col),
                messages::expected_name(what, &t.found()),
            ));
        }
        self.next();
        self.check_reserved(&t, what)?;
        Ok(t.word_str().unwrap().to_string())
    }

    fn parse_program(&mut self) -> Result<Vec<SNode>> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.at_kind(TokKind::Eof) {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<SNode> {
        let t = self.peek(0).clone();
        if t.kind != TokKind::Word {
            return Err(NovaError::new(
                t.line,
                Some(t.col),
                messages::parse::unexpected_start(&t.found()),
            ));
        }
        let w = t.word_str().unwrap().to_lowercase();

        match w.as_str() {
            "use" => return self.p_use(),
            "say" | "write" => return self.p_say(),
            "if" | "unless" => return self.p_if(),
            "repeat" => return self.p_repeat(),
            "stop" => return self.p_stop(),
            "skip" => {
                self.next();
                self.expect_word(&["this"])?;
                self.expect_word(&["one"])?;
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::ContinueStmt, line: t.line });
            }
            "go" => {
                self.next();
                self.expect_word(&["to"])?;
                self.expect_word(&["next"])?;
                self.expect_word(&["turn"])?;
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::ContinueStmt, line: t.line });
            }
            "set" => return self.p_set(),
            "add" => return self.p_addtake("to", true),
            "take" => return self.p_addtake("from", false),
            "remove" => {
                self.next();
                let e = self.parse_expr()?;
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::RemoveStmt { expr: e }, line: t.line });
            }
            "check" => return self.p_check(),
            "try" => return self.p_try(),
            "to" => return self.p_funcdef(),
            "a" | "an"
                if self.at_word_ahead(2, &["is"]) && self.at_word_ahead(4, &["thing"]) =>
            {
                return self.p_thingdef()
            }
            "wait" => return self.p_wait(),
            "pause" => {
                self.next();
                self.expect_word(&["the"])?;
                self.expect_word(&["program"])?;
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::PauseProgram, line: t.line });
            }
            "track" => {
                self.next();
                self.eat_word(&["the"]);
                let name = self.expect_name("variable name")?;
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::TrackStmt { name }, line: t.line });
            }
            "undo" | "redo" => {
                let redo = w == "redo";
                self.next();
                self.expect_word(&["the"])?;
                self.expect_word(&["last"])?;
                self.expect_word(&["change"])?;
                self.expect_word(&["to"])?;
                self.eat_word(&["the"]);
                let name = self.expect_name("variable name")?;
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::UndoStmt { name, redo }, line: t.line });
            }
            "exit" => {
                self.next();
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::StopProgram, line: t.line });
            }
            "when" if self.at_word_ahead(1, &["the"]) && self.at_word_ahead(2, &["program"]) => {
                self.next();
                self.next();
                self.next();
                self.expect_word(&["starts"])?;
                self.expect_eol()?;
                let body = self.p_block(&["done"])?;
                self.expect_word(&["done"])?;
                self.next();
                return Ok(SNode { kind: SKind::WhenProgramStarts { body }, line: t.line });
            }
            "requires" | "ensures" => {
                self.next();
                let e = self.parse_expr()?;
                self.expect_eol()?;
                let kind: &'static str = if w == "requires" { "requires" } else { "ensures" };
                return Ok(SNode { kind: SKind::Contract { kind, expr: e }, line: t.line });
            }
            "give" | "return" => {
                self.next();
                self.eat_word(&["back"]);
                let mut e = None;
                if !self.at_eol() {
                    e = Some(self.parse_expr()?);
                }
                self.expect_eol()?;
                return Ok(SNode { kind: SKind::ReturnStmt { expr: e }, line: t.line });
            }
            "store" => return self.p_store(),
            "the"
                if self.peek(1).kind == TokKind::Word && self.at_word_ahead(2, &["in"]) =>
            {
                return self.p_usemodule()
            }
            _ => {}
        }

        if let Some(st) = self.try_compact_assign()? {
            return Ok(st);
        }

        if let Some(st) = self.try_declare_assign()? {
            return Ok(st);
        }

        let e = self.parse_expr()?;
        self.expect_eol()?;
        Ok(SNode { kind: SKind::ExprStmt { expr: e }, line: t.line })
    }

    fn try_compact_assign(&mut self) -> Result<Option<SNode>> {
        if self.peek(0).kind != TokKind::Word {
            return Ok(None);
        }
        let len = self.toks.len();
        let mut j = self.pos;
        let nxt = self.toks[(j + 1).min(len - 1)].kind;
        if !(nxt == TokKind::Equals || nxt == TokKind::Dot) {
            return Ok(None);
        }
        j += 1;
        while j < len - 1 && self.toks[j].kind == TokKind::Dot && self.toks[j + 1].kind == TokKind::Word
        {
            j += 2;
        }
        if j > len - 1 || self.toks[j].kind != TokKind::Equals {
            return Ok(None);
        }
        let first = self.next();
        self.check_reserved(&first, "variable name")?;
        let mut target = ENode::new(EKind::Var(first.word_str().unwrap().to_string()), first.line);
        while self.at_kind(TokKind::Dot) {
            self.next();
            let fname = self.expect_name("field name")?;
            target = ENode::new(EKind::Field { obj: Box::new(target), name: fname }, first.line);
        }
        self.next();
        let e = self.parse_expr()?;
        self.expect_eol()?;
        Ok(Some(SNode {
            kind: SKind::Assign { target, expr: e },
            line: first.line,
        }))
    }

    fn try_declare_assign(&mut self) -> Result<Option<SNode>> {
        let save = self.pos;
        self.eat_word(&["the"]);
        self.eat_word(&["my"]);
        if self.peek(0).kind == TokKind::Word && self.at_word_ahead(1, &["is"]) {
            let name_t = self.next();
            self.check_reserved(&name_t, "variable name")?;
            self.next();
            let e = self.parse_expr()?;
            self.expect_eol()?;
            let name = name_t.word_str().unwrap().to_string();
            return Ok(Some(SNode {
                kind: SKind::Assign { target: ENode::new(EKind::Var(name), name_t.line), expr: e },
                line: name_t.line,
            }));
        }
        self.pos = save;
        Ok(None)
    }

    fn p_use(&mut self) -> Result<SNode> {
        let t = self.next();
        let mut parts: Vec<String> = Vec::new();
        while !self.at_eol() {
            let tok = self.next();
            parts.push(tok.found());
        }
        Ok(SNode { kind: SKind::UseLib { text: parts.join(" ") }, line: t.line })
    }

    fn p_usemodule(&mut self) -> Result<SNode> {
        let t = self.next();
        let name_t = self.peek(0).clone();
        let is_module = name_t
            .word_str()
            .map(|w| w.ends_with("-module"))
            .unwrap_or(false);
        if name_t.kind != TokKind::Word || !is_module {
            return Err(NovaError::new(
                name_t.line,
                Some(name_t.col),
                messages::parse::module_name_rule(&name_t.found()),
            ));
        }
        self.next();
        self.expect_word(&["in"])?;
        let pt = self.peek(0).clone();
        if pt.kind != TokKind::Str {
            return Err(NovaError::new(
                pt.line,
                Some(pt.col),
                messages::parse::module_path_expected(name_t.word_str().unwrap()),
            ));
        }
        self.next();
        self.expect_eol()?;
        let path = match pt.value {
            TokValue::Text(s) => s,
            _ => unreachable!(),
        };
        Ok(SNode {
            kind: SKind::UseModule { name: name_t.word_str().unwrap().to_string(), path },
            line: t.line,
        })
    }

    fn p_say(&mut self) -> Result<SNode> {
        let t = self.next();
        let newline = t.word_str().unwrap().to_lowercase() == "say";
        let mut exprs = vec![self.parse_expr()?];
        while self.at_word(&["and"]) {
            self.next();
            exprs.push(self.parse_expr()?);
        }
        self.expect_eol()?;
        Ok(SNode { kind: SKind::Say { exprs, newline }, line: t.line })
    }

    fn p_if(&mut self) -> Result<SNode> {
        let t = self.next();
        let cond = self.parse_expr()?;
        self.expect_word(&["then"])?;
        let negate = t.word_str().unwrap().to_lowercase() == "unless";
        let mut branches: Vec<(ENode, SBlock)> = Vec::new();
        let mut els: Option<SBlock> = None;
        let mut used_done = false;

        let first_cond = if negate {
            ENode::new(EKind::NotE(Box::new(cond)), t.line)
        } else {
            cond
        };
        let (body, u) = self.p_body(&["otherwise", "done"])?;
        used_done = used_done || u;
        branches.push((first_cond, body));

        while self.at_word(&["otherwise"]) {
            self.next();
            if self.at_word(&["if"]) {
                self.next();
                let c2 = self.parse_expr()?;
                self.expect_word(&["then"])?;
                let (b2, u2) = self.p_body(&["otherwise", "done"])?;
                used_done = used_done || u2;
                branches.push((c2, b2));
                continue;
            }
            let (b3, u3) = self.p_body(&["done"])?;
            used_done = used_done || u3;
            els = Some(b3);
            break;
        }

        if used_done {
            self.expect_word(&["done"])?;
            self.next();
        } else if self.at_word(&["done"]) {
            self.next();
        }
        Ok(SNode { kind: SKind::If { branches, otherwise: els }, line: t.line })
    }

    fn p_body(&mut self, stops: &[&str]) -> Result<(SBlock, bool)> {
        if self.at_kind(TokKind::Newline) {
            self.skip_newlines();
            let mut stmts = Vec::new();
            loop {
                if self.at_kind(TokKind::Eof) {
                    let mut sorted: Vec<&str> = stops.to_vec();
                    sorted.sort_unstable();
                    return Err(self.err_here(messages::parse::block_unclosed(&sorted.join("/"))));
                }
                if self.at_kind(TokKind::Newline) {
                    self.next();
                    continue;
                }
                if self.at_word(stops) {
                    let line = self.peek(0).line;
                    return Ok((SBlock { stmts, line }, true));
                }
                stmts.push(self.parse_statement()?);
            }
        } else {
            let stmts = vec![self.parse_statement()?];
            let line = self.peek(0).line;
            Ok((SBlock { stmts, line }, false))
        }
    }

    fn p_block(&mut self, stops: &[&str]) -> Result<SBlock> {
        self.skip_newlines();
        let mut stmts = Vec::new();
        loop {
            if self.at_kind(TokKind::Eof) {
                return Err(self.err_here(messages::parse::block_missing_done()));
            }
            if self.at_kind(TokKind::Newline) {
                self.next();
                continue;
            }
            if self.at_word(stops) {
                let line = self.peek(0).line;
                return Ok(SBlock { stmts, line });
            }
            stmts.push(self.parse_statement()?);
        }
    }

    fn p_repeat(&mut self) -> Result<SNode> {
        let t = self.next();
        if self.eat_word(&["forever"]).is_some() {
            let body = self.p_block(&["done"])?;
            self.expect_word(&["done"])?;
            self.next();
            return Ok(SNode { kind: SKind::RepeatForever { body }, line: t.line });
        }
        if self.eat_word(&["until"]).is_some() {
            let cond = self.parse_expr()?;
            self.expect_eol()?;
            let body = self.p_block(&["done"])?;
            self.expect_word(&["done"])?;
            self.next();
            return Ok(SNode { kind: SKind::RepeatUntil { cond, body }, line: t.line });
        }
        if self.eat_word(&["while"]).is_some() {
            let cond = self.parse_expr()?;
            self.expect_eol()?;
            let body = self.p_block(&["done"])?;
            self.expect_word(&["done"])?;
            self.next();
            return Ok(SNode { kind: SKind::RepeatWhile { cond, body }, line: t.line });
        }
        if self.at_word(&["each", "for"])
            && (self.at_word(&["each"]) || self.at_word_ahead(1, &["each"]))
        {
            self.eat_word(&["for"]);
            self.expect_word(&["each"])?;
            let var = self.expect_name("loop variable")?;
            self.expect_word(&["in"])?;
            let iterable = self.parse_expr()?;
            self.expect_eol()?;
            let body = self.p_block(&["done"])?;
            self.expect_word(&["done"])?;
            self.next();
            return Ok(SNode { kind: SKind::RepeatEach { var, iterable, body }, line: t.line });
        }
        if self.at_word(&["with"]) {
            self.next();
            let var = self.expect_name("loop variable")?;
            self.expect_word(&["from"])?;
            let start = self.parse_term_first_only()?;
            self.expect_word(&["to"])?;
            let end = self.parse_term_first_only()?;
            self.expect_eol()?;
            let body = self.p_block(&["done"])?;
            self.expect_word(&["done"])?;
            self.next();
            return Ok(SNode {
                kind: SKind::RepeatCounting { var, start, end, body },
                line: t.line,
            });
        }
        let count = self.parse_term_first_only()?;
        self.expect_word(&["times"])?;
        self.expect_eol()?;
        let body = self.p_block(&["done"])?;
        self.expect_word(&["done"])?;
        self.next();
        Ok(SNode { kind: SKind::RepeatTimes { count, body }, line: t.line })
    }

    fn p_stop(&mut self) -> Result<SNode> {
        let t = self.next();
        self.eat_word(&["the"]);
        let w = self.expect_word(&["loop", "program"])?;
        self.expect_eol()?;
        Ok(if w == "loop" {
            SNode { kind: SKind::BreakStmt, line: t.line }
        } else {
            SNode { kind: SKind::StopProgram, line: t.line }
        })
    }

    fn p_set(&mut self) -> Result<SNode> {
        let t = self.next();
        let target = self.parse_lvalue()?;
        self.expect_word(&["to"])?;
        let e = self.parse_expr()?;
        self.expect_eol()?;
        Ok(SNode { kind: SKind::Assign { target, expr: e }, line: t.line })
    }

    fn parse_lvalue(&mut self) -> Result<ENode> {
        let t0 = self.peek(0).clone();
        if self.at_word(&["the"]) {
            let node = self.parse_the_chain()?;
            if matches!(node.kind, EKind::Field { .. }) {
                return Ok(node);
            }
            return Err(self.err_here(messages::parse::set_the_form()));
        }
        self.eat_word(&["my"]);
        if self.peek(0).kind != TokKind::Word {
            return Err(self.err_here(messages::parse::lvalue_name(&t0.found())));
        }
        let nt = self.next();
        self.check_reserved(&nt, "variable name")?;
        let name = nt.word_str().unwrap().to_string();
        if self.at_word(&["of"]) {
            self.next();
            let obj = self.parse_arith()?;
            return Ok(ENode::new(EKind::Field { obj: Box::new(obj), name }, nt.line));
        }
        Ok(ENode::new(EKind::Var(name), nt.line))
    }

    fn parse_the_chain(&mut self) -> Result<ENode> {
        let t = self.peek(0).clone();
        self.next();
        if self.peek(0).kind != TokKind::Word {
            let f = self.peek(0).found();
            return Err(NovaError::new(
                self.peek(0).line,
                Some(self.peek(0).col),
                messages::parse::expected_name_after_the(&f),
            ));
        }
        let head_t = self.next();
        let w = head_t.word_str().unwrap().to_lowercase();

        if w == "contents" {
            self.eat_word(&["of"]);
            let src = self.parse_arith()?;
            let mut as_json = false;
            if self.at_word(&["parsed"]) {
                self.next();
                self.expect_word(&["as"])?;
                self.expect_word(&["json"])?;
                as_json = true;
            }
            return Ok(ENode::new(EKind::ContentsOf { e: Box::new(src), as_json }, t.line));
        }
        if w == "first" && self.at_word(&["item"]) {
            self.next();
            self.expect_word(&["of"])?;
            let e = self.parse_factor()?;
            return Ok(ENode::new(EKind::FirstItem(Box::new(e)), t.line));
        }
        if w == "last" && self.at_word(&["item"]) {
            self.next();
            self.expect_word(&["of"])?;
            let e = self.parse_factor()?;
            return Ok(ENode::new(EKind::LastItem(Box::new(e)), t.line));
        }
        if w == "number" && self.at_word(&["value"]) {
            self.next();
            self.expect_word(&["of"])?;
            let e = self.parse_factor()?;
            return Ok(ENode::new(EKind::NumVal(Box::new(e)), t.line));
        }
        if w == "length" {
            self.eat_word(&["of"]);
            let e = self.parse_factor()?;
            return Ok(ENode::new(EKind::CountOf(Box::new(e)), t.line));
        }
        if self.at_word(&["of"]) {
            self.next();
            let obj = self.parse_arith()?;
            let name = head_t.word_str().unwrap().to_string();
            return Ok(ENode::new(EKind::Field { obj: Box::new(obj), name }, t.line));
        }
        Ok(ENode::new(EKind::Var(head_t.word_str().unwrap().to_string()), t.line))
    }

    fn p_addtake(&mut self, prep: &str, is_add: bool) -> Result<SNode> {
        let t = self.next();
        let e = self.parse_arith()?;
        self.expect_word(&[prep])?;
        self.eat_word(&["the"]);
        let name = self.expect_name("variable name")?;
        self.expect_eol()?;
        Ok(if is_add {
            SNode { kind: SKind::AddTo { name, expr: e }, line: t.line }
        } else {
            SNode { kind: SKind::TakeFrom { name, expr: e }, line: t.line }
        })
    }

    fn p_check(&mut self) -> Result<SNode> {
        let t = self.next();
        let subject = self.parse_expr()?;
        let mut arms: Vec<CheckArm> = Vec::new();
        let mut els: Option<SBlock> = None;
        loop {
            if matches!(self.peek(0).kind, TokKind::Newline | TokKind::Eof) {
                if self.at_kind(TokKind::Eof) {
                    return Err(self.err_here(messages::parse::check_missing_done()));
                }
                self.next();
                continue;
            }
            if self.at_word(&["when"]) {
                let wt = self.next();
                self.eat_word(&["it"]);
                self.eat_word(&["is"]);
                let (kind, val, neg) = self.parse_pattern(wt.line)?;
                let (body, _) = self.p_body(&["when", "otherwise", "done"])?;
                arms.push(CheckArm { kind, val, neg, body });
                continue;
            }
            if self.at_word(&["otherwise"]) {
                self.next();
                let (b, _) = self.p_body(&["when", "done"])?;
                els = Some(b);
            }
            break;
        }
        if self.at_word(&["done"]) {
            self.next();
        }
        Ok(SNode { kind: SKind::Check { subject, arms, otherwise: els }, line: t.line })
    }

    fn parse_pattern(&mut self, _line: usize) -> Result<(&'static str, Option<ENode>, bool)> {
        let neg = self.eat_word(&["not"]).is_some();
        if self.at_word(&["a", "an"]) {
            self.next();
            self.expect_word(&["number"])?;
            return Ok(("isnum", None, neg));
        }
        if self.at_word(&["equal"]) {
            self.next();
            self.expect_word(&["to"])?;
            let e = self.parse_arith()?;
            return Ok(("eq", Some(e), neg));
        }
        if self.at_word(&["the"]) && self.at_word_ahead(1, &["same"]) {
            self.next();
            self.next();
            self.expect_word(&["as"])?;
            let e = self.parse_arith()?;
            return Ok(("eq", Some(e), neg));
        }
        if self.at_word(&["starts"]) {
            self.next();
            self.expect_word(&["with"])?;
            let e = self.parse_arith()?;
            return Ok(("startswith", Some(e), neg));
        }
        if self.at_word(&["ends"]) {
            self.next();
            self.expect_word(&["with"])?;
            let e = self.parse_arith()?;
            return Ok(("endswith", Some(e), neg));
        }
        if self.at_word(&["contains"]) {
            self.next();
            let e = self.parse_arith()?;
            return Ok(("contains", Some(e), neg));
        }
        if self.at_word(&["empty"]) {
            self.next();
            return Ok(("isempty", None, neg));
        }
        let e = self.parse_arith()?;
        Ok(("eq", Some(e), neg))
    }

    fn p_try(&mut self) -> Result<SNode> {
        let t = self.next();
        let body = self.p_block(&["if", "done"])?;
        let mut errname = None;
        let mut handler = None;
        if self.at_word(&["if"]) {
            self.next();
            self.expect_word(&["it"])?;
            self.expect_word(&["fails"])?;
            if self.eat_word(&["as"]).is_some() {
                errname = Some(self.expect_name("variable name")?);
            }
            handler = Some(self.p_block(&["done"])?);
        }
        self.expect_word(&["done"])?;
        self.next();
        Ok(SNode { kind: SKind::TryStmt { body, errname, handler }, line: t.line })
    }

    fn p_funcdef(&mut self) -> Result<SNode> {
        let t = self.next();
        let name = self.expect_name("function name")?;
        let mut params = Vec::new();
        if self.eat_word(&["with"]).is_some() {
            loop {
                params.push(self.expect_name("parameter name")?);
                if self.eat_word(&["and"]).is_none() {
                    break;
                }
            }
        }
        self.expect_eol()?;
        let body = self.p_block(&["done"])?;
        self.expect_word(&["done"])?;
        self.next();
        Ok(SNode { kind: SKind::FuncDef { name, params, body }, line: t.line })
    }

    fn p_thingdef(&mut self) -> Result<SNode> {
        let t = self.next();
        let name = self.expect_name("thing name")?;
        self.expect_word(&["is"])?;
        self.expect_word(&["a"])?;
        self.expect_word(&["thing"])?;
        self.expect_word(&["with"])?;
        let mut fields: Vec<(String, Option<ENode>)> = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_word(&["done"]) {
                break;
            }
            self.eat_word(&["a"]);
            self.eat_word(&["an"]);
            let fname = self.expect_name("field name")?;
            let mut default = None;
            if self.at_word(&["set"]) {
                self.next();
                self.expect_word(&["to"])?;
                default = Some(self.parse_expr()?);
            } else {
                self.expect_eol()?;
            }
            fields.push((fname, default));
        }
        self.expect_word(&["done"])?;
        self.next();
        Ok(SNode { kind: SKind::ThingDef { name, fields }, line: t.line })
    }

    fn p_wait(&mut self) -> Result<SNode> {
        let t = self.next();
        let amount = self.parse_term_first_only()?;
        let unit_w = self.next().word_str().unwrap_or_default().to_lowercase();
        let unit: &'static str = match unit_w.as_str() {
            "second" | "seconds" => "seconds",
            "minute" | "minutes" => "minutes",
            "hour" | "hours" => "hours",
            "ms" | "millisecond" | "milliseconds" => "milliseconds",
            _ => return Err(self.err_here(messages::parse::unknown_time_unit(&unit_w))),
        };
        self.expect_eol()?;
        Ok(SNode { kind: SKind::WaitStmt { amount, unit }, line: t.line })
    }

    fn p_store(&mut self) -> Result<SNode> {
        let t = self.next();
        let value = self.parse_arith()?;
        self.expect_word(&["in"])?;
        let path = self.parse_arith()?;
        if self.at_word(&["as"]) {
            self.next();
            self.expect_word(&["json"])?;
        }
        self.expect_eol()?;
        Ok(SNode { kind: SKind::StoreJson { value, path }, line: t.line })
    }

    fn parse_expr(&mut self) -> Result<ENode> {
        let e = self.parse_or()?;
        Ok(wrap_optional(e))
    }

    fn parse_or(&mut self) -> Result<ENode> {
        let mut left = self.parse_and()?;
        loop {
            let is_or = self.at_word(&["or"]) || self.at_kind(TokKind::PipePipe);
            if !is_or {
                break;
            }
            let t = self.next();
            let right = self.parse_and()?;
            left = ENode::new(EKind::Bin { op: "or", l: Box::new(left), r: Box::new(right) }, t.line);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ENode> {
        let mut left = self.parse_comparison()?;
        loop {
            let is_and = self.at_word(&["and"]) || self.at_kind(TokKind::AmpAmp);
            if !is_and {
                break;
            }
            let t = self.next();
            let right = self.parse_comparison()?;
            left = ENode::new(EKind::Bin { op: "and", l: Box::new(left), r: Box::new(right) }, t.line);
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<ENode> {
        let mut left = self.parse_arith()?;
        loop {
            let t = self.peek(0).clone();
            let sym_op: Option<&'static str> = match t.kind {
                TokKind::EqualEqual => Some("eq"),
                TokKind::BangEqual => Some("ne"),
                TokKind::Lt => Some("lt"),
                TokKind::Lte => Some("lte"),
                TokKind::Gt => Some("gt"),
                TokKind::Gte => Some("gte"),
                _ => None,
            };
            if let Some(op) = sym_op {
                self.next();
                let right = self.parse_arith()?;
                left = ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, t.line);
                continue;
            }
            if t.kind != TokKind::Word {
                break;
            }
            let w = t.word_str().unwrap().to_lowercase();
            match w.as_str() {
                "is" => {
                    self.next();
                    left = self.parse_is_tail(left, t.line)?;
                    continue;
                }
                "contains" => {
                    self.next();
                    let right = self.parse_arith()?;
                    left = ENode::new(
                        EKind::Bin { op: "contains", l: Box::new(left), r: Box::new(right) },
                        t.line,
                    );
                    continue;
                }
                "starts" => {
                    self.next();
                    self.expect_word(&["with"])?;
                    let right = self.parse_arith()?;
                    left = ENode::new(
                        EKind::Bin { op: "startswith", l: Box::new(left), r: Box::new(right) },
                        t.line,
                    );
                    continue;
                }
                "ends" => {
                    self.next();
                    self.expect_word(&["with"])?;
                    let right = self.parse_arith()?;
                    left = ENode::new(
                        EKind::Bin { op: "endswith", l: Box::new(left), r: Box::new(right) },
                        t.line,
                    );
                    continue;
                }
                "has" if self.at_word_ahead(1, &["no"]) => {
                    self.next();
                    self.next();
                    self.expect_word(&["items"])?;
                    left = ENode::new(EKind::HasNoItems(Box::new(left)), t.line);
                    continue;
                }
                "exists" => {
                    self.next();
                    left = ENode::new(EKind::ExistsE { e: Box::new(left), flag: true }, t.line);
                    continue;
                }
                "does" if self.at_word_ahead(1, &["not"]) => {
                    self.next();
                    self.next();
                    self.expect_word(&["exist"])?;
                    left = ENode::new(EKind::ExistsE { e: Box::new(left), flag: false }, t.line);
                    continue;
                }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_is_tail(&mut self, left: ENode, line: usize) -> Result<ENode> {
        let neg = self.eat_word(&["not"]).is_some();
        let eqne = |neg: bool| if neg { "ne" } else { "eq" };

        if self.at_word(&["a", "an"]) {
            self.next();
            self.expect_word(&["number"])?;
            return Ok(ENode::new(EKind::IsNumberTest { e: Box::new(left), negate: neg }, line));
        }
        if self.eat_word(&["nothing"]).is_some() {
            let op: &'static str = eqne(neg);
            return Ok(ENode::new(
                EKind::Bin {
                    op,
                    l: Box::new(left),
                    r: Box::new(ENode::new(EKind::Lit(PyLit::Nothing), line)),
                },
                line,
            ));
        }
        if self.eat_word(&["empty"]).is_some() {
            let e = ENode::new(EKind::IsEmptyE(Box::new(left)), line);
            return Ok(if neg { ENode::new(EKind::NotE(Box::new(e)), line) } else { e });
        }
        if self.eat_word(&["true"]).is_some() {
            let op: &'static str = eqne(neg);
            return Ok(ENode::new(
                EKind::Bin {
                    op,
                    l: Box::new(left),
                    r: Box::new(ENode::new(EKind::Lit(PyLit::Bool(true)), line)),
                },
                line,
            ));
        }
        if self.eat_word(&["false"]).is_some() {
            let op: &'static str = eqne(neg);
            return Ok(ENode::new(
                EKind::Bin {
                    op,
                    l: Box::new(left),
                    r: Box::new(ENode::new(EKind::Lit(PyLit::Bool(false)), line)),
                },
                line,
            ));
        }
        if self.at_word(&["equal"]) {
            self.next();
            self.expect_word(&["to"])?;
            let right = self.parse_arith()?;
            let op: &'static str = eqne(neg);
            return Ok(ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, line));
        }
        if self.at_word(&["the"]) && self.at_word_ahead(1, &["same"]) {
            self.next();
            self.next();
            self.expect_word(&["as"])?;
            let right = self.parse_arith()?;
            let op: &'static str = eqne(neg);
            return Ok(ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, line));
        }
        if self.at_word(&["greater"]) {
            self.next();
            self.expect_word(&["than"])?;
            let right = self.parse_arith()?;
            let op: &'static str = if neg { "lte" } else { "gt" };
            return Ok(ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, line));
        }
        if self.at_word(&["less"]) {
            self.next();
            self.expect_word(&["than"])?;
            let right = self.parse_arith()?;
            let op: &'static str = if neg { "gte" } else { "lt" };
            return Ok(ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, line));
        }
        if self.at_word(&["at"]) {
            self.next();
            let w2 = self.expect_word(&["least", "most"])?;
            let right = self.parse_arith()?;
            let op: &'static str =
                if w2 == "least" { if neg { "lt" } else { "gte" } } else { if neg { "gt" } else { "lte" } };
            return Ok(ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, line));
        }
        let right = self.parse_arith()?;
        let op: &'static str = eqne(neg);
        Ok(ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, line))
    }

    fn parse_arith(&mut self) -> Result<ENode> {
        let mut left = self.parse_term()?;
        loop {
            let op: Option<&'static str> = if self.at_word(&["plus"]) || self.at_kind(TokKind::Plus)
            {
                Some("plus")
            } else if self.at_word(&["minus"]) || self.at_kind(TokKind::Minus) {
                Some("minus")
            } else {
                None
            };
            let Some(op) = op else { break };
            let t = self.next();
            let right = self.parse_term()?;
            left = ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, t.line);
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<ENode> {
        let mut left = self.parse_factor()?;
        loop {
            enum Tail {
                Times,
                MultipliedBy,
                DividedBy,
                Over,
                Mod,
                Slash,
            }
            let tail: Option<Tail> = if self.at_word(&["times"]) || self.at_kind(TokKind::Star) {
                Some(Tail::Times)
            } else if self.at_word(&["multiplied"]) && self.at_word_ahead(1, &["by"]) {
                Some(Tail::MultipliedBy)
            } else if self.at_word(&["divided"]) && self.at_word_ahead(1, &["by"]) {
                Some(Tail::DividedBy)
            } else if self.at_word(&["over"]) {
                Some(Tail::Over)
            } else if self.at_word(&["mod"]) || self.at_kind(TokKind::Percent) {
                Some(Tail::Mod)
            } else if self.at_kind(TokKind::Slash) {
                Some(Tail::Slash)
            } else {
                None
            };
            let Some(tail) = tail else { break };
            let t = self.next();
            if matches!(tail, Tail::MultipliedBy | Tail::DividedBy) {
                self.next();
            }
            let op: &'static str = match tail {
                Tail::Times | Tail::MultipliedBy => "times",
                Tail::DividedBy | Tail::Over | Tail::Slash => "divided",
                Tail::Mod => "mod",
            };
            let right = self.parse_factor()?;
            left = ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, t.line);
        }
        Ok(left)
    }

    fn parse_term_first_only(&mut self) -> Result<ENode> {
        let mut left = self.parse_factor()?;
        while self.at_word(&["plus", "minus"]) {
            let t = self.next();
            let op: &'static str =
                if t.word_str().unwrap().to_lowercase() == "plus" { "plus" } else { "minus" };
            let right = self.parse_factor()?;
            left = ENode::new(EKind::Bin { op, l: Box::new(left), r: Box::new(right) }, t.line);
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<ENode> {
        if self.at_word(&["not"]) {
            let t = self.next();
            let e = self.parse_comparison()?;
            return Ok(ENode::new(EKind::NotE(Box::new(e)), t.line));
        }
        if self.at_kind(TokKind::Bang) {
            let t = self.next();
            let e = self.parse_factor()?;
            return Ok(ENode::new(EKind::NotE(Box::new(e)), t.line));
        }
        if self.at_kind(TokKind::Minus) {
            let t = self.next();
            let e = self.parse_factor()?;
            return Ok(ENode::new(
                EKind::Bin {
                    op: "minus",
                    l: Box::new(ENode::new(EKind::Lit(PyLit::Int("0".to_string())), t.line)),
                    r: Box::new(e),
                },
                t.line,
            ));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<ENode> {
        let mut e = self.parse_primary()?;
        loop {
            if self.at_kind(TokKind::Dot) {
                let dot_t = self.next();
                let name_t = self.peek(0).clone();
                if name_t.kind != TokKind::Word {
                    return Err(NovaError::new(
                        name_t.line,
                        Some(name_t.col),
                        messages::expected_name("field name after '.'", &name_t.found()),
                    ));
                }
                self.next();
                let name = name_t.word_str().unwrap().to_string();
                if self.at_kind(TokKind::LParen) {
                    let base_var = match &e.kind {
                        EKind::Var(v) => Some(v.clone()),
                        _ => None,
                    };
                    let Some(base_var) = base_var else {
                        return Err(NovaError::new(
                            dot_t.line,
                            Some(dot_t.col),
                            messages::parse::method_call_later(),
                        ));
                    };
                    self.next();
                    let mut args = Vec::new();
                    if !self.at_kind(TokKind::RParen) {
                        args.push(self.parse_arith()?);
                        while self.at_kind(TokKind::Comma) {
                            self.next();
                            args.push(self.parse_arith()?);
                        }
                    }
                    if !self.at_kind(TokKind::RParen) {
                        return Err(self.err_here(messages::parse::missing_rparen()));
                    }
                    self.next();
                    e = ENode::new(EKind::ModuleCall { module: base_var, name, args }, dot_t.line);
                } else {
                    e = ENode::new(EKind::Field { obj: Box::new(e), name }, dot_t.line);
                }
            } else if self.at_kind(TokKind::Question) {
                let t = self.next();
                e = ENode::new(EKind::QuestionE(Box::new(e)), t.line);
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// Returns Some((params, tokens-to-consume)) when the upcoming tokens form
    /// a lambda header: `ident =>` or `( ident , ident* ) =>`.
    fn lambda_header_ahead(&self) -> Option<(Vec<String>, usize)> {
        let mut k = 0usize;
        if self.peek(k).kind == TokKind::Word {
            if self.peek(k + 1).kind == TokKind::FatArrow {
                let name = self.peek(k).word_str().unwrap().to_string();
                return Some((vec![name], k + 2));
            }
            return None;
        }
        if self.peek(k).kind != TokKind::LParen {
            return None;
        }
        k += 1;
        let mut params = Vec::new();
        loop {
            let t = self.peek(k);
            if t.kind == TokKind::Word {
                // reject reserved-ish? keep simple: any word is a param name
                params.push(t.word_str().unwrap().to_string());
                k += 1;
                match self.peek(k).kind {
                    TokKind::Comma => { k += 1; }
                    TokKind::RParen => {
                        k += 1;
                        break;
                    }
                    _ => return None,
                }
            } else {
                return None;
            }
        }
        if self.peek(k).kind != TokKind::FatArrow || params.is_empty() {
            return None;
        }
        Some((params, k + 1))
    }

    fn parse_primary(&mut self) -> Result<ENode> {
        let t = self.peek(0).clone();
        let line = t.line;

        // C10 lambda: `x => expr` or `(a, b) => expr` (compact skin)
        if let Some((params, after_arrow)) = self.lambda_header_ahead() {
            for _ in 0..after_arrow {
                self.next();
            }
            let body = self.parse_expr()?;
            return Ok(ENode::new(EKind::Lambda { params, body: Box::new(body) }, line));
        }

        match t.kind {
            TokKind::Number => {
                self.next();
                let lit = match t.value {
                    TokValue::Num(NumLit::Int(i)) => PyLit::Int(i),
                    TokValue::Num(NumLit::Float(f)) => PyLit::Float(f),
                    _ => unreachable!(),
                };
                return Ok(ENode::new(EKind::Lit(lit), line));
            }
            TokKind::Str => {
                self.next();
                let raw = match t.value {
                    TokValue::Text(s) => s,
                    _ => unreachable!(),
                };
                return Ok(ENode::new(EKind::StrLit(raw), line));
            }
            TokKind::LParen => {
                self.next();
                let e = self.parse_expr()?;
                if !self.at_kind(TokKind::RParen) {
                    return Err(self.err_here(messages::parse::missing_rparen()));
                }
                self.next();
                return Ok(e);
            }
            TokKind::LBracket => {
                self.next();
                let mut items = Vec::new();
                while !self.at_kind(TokKind::RBracket) {
                    items.push(self.parse_expr()?);
                    if self.at_kind(TokKind::Comma) {
                        self.next();
                    } else if !self.at_kind(TokKind::RBracket) {
                        return Err(self.err_here(messages::parse::list_sep()));
                    }
                }
                self.next();
                return Ok(ENode::new(EKind::ListLit(items), line));
            }
            TokKind::Word => {}
            _ => {
                return Err(self.err_here(messages::parse::unexpected_in_expr(&t.found())));
            }
        }

        let w = t.word_str().unwrap().to_lowercase();
        match w.as_str() {
            "true" => {
                self.next();
                Ok(ENode::new(EKind::Lit(PyLit::Bool(true)), line))
            }
            "false" => {
                self.next();
                Ok(ENode::new(EKind::Lit(PyLit::Bool(false)), line))
            }
            "nothing" | "none" | "null" => {
                self.next();
                Ok(ENode::new(EKind::Lit(PyLit::Nothing), line))
            }
            "ask" => {
                self.next();
                let prompt = self.parse_arith()?;
                Ok(ENode::new(EKind::AskE(Box::new(prompt)), line))
            }
            "a" | "an" if self.at_word_ahead(1, &["random"]) => {
                self.next();
                self.next();
                self.expect_word(&["number"])?;
                self.expect_word(&["between"])?;
                let a = self.parse_term_first_only()?;
                self.expect_word(&["and"])?;
                let b = self.parse_term_first_only()?;
                Ok(ENode::new(EKind::RandomBetween { a: Box::new(a), b: Box::new(b) }, line))
            }
            "a" | "an" if self.at_word_ahead(1, &["empty"]) && self.at_word_ahead(2, &["list"]) => {
                self.next();
                self.next();
                self.next();
                Ok(ENode::new(EKind::EmptyListE, line))
            }
            "a" | "an" if self.at_word_ahead(1, &["copy"]) => {
                self.next();
                self.next();
                self.expect_word(&["of"])?;
                let e = self.parse_arith()?;
                Ok(ENode::new(EKind::CopyOf(Box::new(e)), line))
            }
            "a" | "an" if self.at_word_ahead(1, &["new"]) => {
                self.next();
                self.next();
                let cls_t = self.next();
                self.check_reserved(&cls_t, "thing name")?;
                let cls = cls_t.word_str().unwrap().to_string();
                let mut setters: Vec<(String, ENode)> = Vec::new();
                if self.eat_word(&["with"]).is_some() {
                    loop {
                        let ft = self.next();
                        let fname = ft.found();
                        self.expect_word(&["set"])?;
                        self.expect_word(&["to"])?;
                        let v = self.parse_arith()?;
                        setters.push((fname, v));
                        if self.at_word(&["and"]) && self.at_word_ahead(2, &["set"]) {
                            self.next();
                            continue;
                        }
                        break;
                    }
                }
                Ok(ENode::new(EKind::NewThing { cls, setters }, line))
            }
            "how" if self.at_word_ahead(1, &["many"]) => {
                self.next();
                self.next();
                self.expect_word(&["items"])?;
                self.expect_word(&["are"])?;
                self.expect_word(&["in"])?;
                let e = self.parse_factor()?;
                Ok(ENode::new(EKind::CountOf(Box::new(e)), line))
            }
            "everything" => {
                self.next();
                self.expect_word(&["after"])?;
                let sep = self.parse_arith()?;
                self.expect_word(&["in"])?;
                self.eat_word(&["the"]);
                let e = self.parse_arith()?;
                Ok(ENode::new(EKind::EverythingAfter { sep: Box::new(sep), e: Box::new(e) }, line))
            }
            "every" if self.at_word_ahead(1, &["item"]) => {
                self.next();
                self.next();
                self.expect_word(&["of"])?;
                let src = self.parse_arith()?;
                self.expect_word(&["turned"])?;
                self.expect_word(&["into"])?;
                self.eat_word(&["a"]);
                self.eat_word(&["an"]);
                let thing_t = self.next();
                let thing = thing_t.found();
                Ok(ENode::new(EKind::EveryTurnedInto { e: Box::new(src), thing }, line))
            }
            "item" => {
                self.next();
                let idx = self.parse_term_first_only()?;
                self.expect_word(&["of"])?;
                let e = self.parse_arith()?;
                Ok(ENode::new(EKind::ItemAt { idx: Box::new(idx), e: Box::new(e) }, line))
            }
            "the" => self.parse_the_chain(),
            "not" => {
                self.next();
                let e = self.parse_comparison()?;
                Ok(ENode::new(EKind::NotE(Box::new(e)), line))
            }
            _ => self.parse_bare(),
        }
    }

    fn parse_bare(&mut self) -> Result<ENode> {
        let t = self.peek(0).clone();
        if t.kind != TokKind::Word {
            return Err(self.err_here(messages::expected_name("name", &t.found())));
        }
        let nt = self.next();
        let name = nt.word_str().unwrap().to_string();
        if self.at_kind(TokKind::LParen) {
            self.next();
            let mut args = Vec::new();
            if !self.at_kind(TokKind::RParen) {
                args.push(self.parse_arith()?);
                while self.at_kind(TokKind::Comma) {
                    self.next();
                    args.push(self.parse_arith()?);
                }
            }
            if !self.at_kind(TokKind::RParen) {
                return Err(self.err_here(messages::parse::missing_rparen()));
            }
            self.next();
            return Ok(ENode::new(EKind::Call { name, args }, nt.line));
        }
        if self.at_word(&["with"]) {
            self.next();
            let mut args = vec![self.parse_arith()?];
            while self.at_word(&["and"]) {
                self.next();
                args.push(self.parse_arith()?);
            }
            return Ok(ENode::new(EKind::Call { name, args }, nt.line));
        }
        Ok(ENode::new(EKind::Var(name), nt.line))
    }
}

fn has_q(e: &ENode) -> bool {
    match &e.kind {
        EKind::QuestionE(_) => true,
        EKind::ListLit(items) => items.iter().any(has_q),
        EKind::Field { obj, .. }
        | EKind::NotE(obj)
        | EKind::NumVal(obj)
        | EKind::CountOf(obj)
        | EKind::FirstItem(obj)
        | EKind::LastItem(obj)
        | EKind::IsEmptyE(obj)
        | EKind::HasNoItems(obj)
        | EKind::CopyOf(obj)
        | EKind::AskE(obj) => has_q(obj),
        EKind::Bin { l, r, .. } => has_q(l) || has_q(r),
        EKind::Call { args, .. } | EKind::ModuleCall { args, .. } => args.iter().any(has_q),
        EKind::NewThing { setters, .. } => setters.iter().any(|(_, v)| has_q(v)),
        EKind::EverythingAfter { sep, e } => has_q(sep) || has_q(e),
        EKind::ItemAt { idx, e } => has_q(idx) || has_q(e),
        EKind::ExistsE { e, .. } | EKind::IsNumberTest { e, .. } => has_q(e),
        EKind::RandomBetween { a, b } => has_q(a) || has_q(b),
        EKind::ContentsOf { e, .. } => has_q(e),
        EKind::EveryTurnedInto { e, .. } => has_q(e),
        _ => false,
    }
}

fn strip_q(e: ENode) -> ENode {
    let rebuild = |k: EKind| ENode { kind: k, line: e.line };
    match e.kind {
        EKind::QuestionE(inner) => strip_q(*inner),
        EKind::ListLit(items) => rebuild(EKind::ListLit(items.into_iter().map(strip_q).collect())),
        EKind::Field { obj, name } => {
            rebuild(EKind::Field { obj: Box::new(strip_q(*obj)), name })
        }
        EKind::Bin { op, l, r } => {
            rebuild(EKind::Bin { op, l: Box::new(strip_q(*l)), r: Box::new(strip_q(*r)) })
        }
        EKind::NotE(inner) => rebuild(EKind::NotE(Box::new(strip_q(*inner)))),
        EKind::Call { name, args } => {
            rebuild(EKind::Call { name, args: args.into_iter().map(strip_q).collect() })
        }
        EKind::ModuleCall { module, name, args } => rebuild(EKind::ModuleCall {
            module,
            name,
            args: args.into_iter().map(strip_q).collect(),
        }),
        EKind::NewThing { cls, setters } => rebuild(EKind::NewThing {
            cls,
            setters: setters.into_iter().map(|(k, v)| (k, strip_q(v))).collect(),
        }),
        EKind::NumVal(inner) => rebuild(EKind::NumVal(Box::new(strip_q(*inner)))),
        EKind::EverythingAfter { sep, e } => rebuild(EKind::EverythingAfter {
            sep: Box::new(strip_q(*sep)),
            e: Box::new(strip_q(*e)),
        }),
        EKind::CountOf(inner) => rebuild(EKind::CountOf(Box::new(strip_q(*inner)))),
        EKind::ItemAt { idx, e } => rebuild(EKind::ItemAt {
            idx: Box::new(strip_q(*idx)),
            e: Box::new(strip_q(*e)),
        }),
        EKind::FirstItem(inner) => rebuild(EKind::FirstItem(Box::new(strip_q(*inner)))),
        EKind::LastItem(inner) => rebuild(EKind::LastItem(Box::new(strip_q(*inner)))),
        EKind::IsEmptyE(inner) => rebuild(EKind::IsEmptyE(Box::new(strip_q(*inner)))),
        EKind::HasNoItems(inner) => rebuild(EKind::HasNoItems(Box::new(strip_q(*inner)))),
        EKind::ExistsE { e, flag } => {
            rebuild(EKind::ExistsE { e: Box::new(strip_q(*e)), flag })
        }
        EKind::IsNumberTest { e, negate } => {
            rebuild(EKind::IsNumberTest { e: Box::new(strip_q(*e)), negate })
        }
        EKind::RandomBetween { a, b } => rebuild(EKind::RandomBetween {
            a: Box::new(strip_q(*a)),
            b: Box::new(strip_q(*b)),
        }),
        EKind::ContentsOf { e, as_json } => {
            rebuild(EKind::ContentsOf { e: Box::new(strip_q(*e)), as_json })
        }
        EKind::EveryTurnedInto { e, thing } => {
            rebuild(EKind::EveryTurnedInto { e: Box::new(strip_q(*e)), thing })
        }
        EKind::CopyOf(inner) => rebuild(EKind::CopyOf(Box::new(strip_q(*inner)))),
        EKind::AskE(inner) => rebuild(EKind::AskE(Box::new(strip_q(*inner)))),
        other => rebuild(other),
    }
}

fn wrap_optional(mut e: ENode) -> ENode {
    if !has_q(&e) {
        return e;
    }
    let mut stripped = ENode { kind: EKind::Lit(PyLit::Nothing), line: 0 };
    std::mem::swap(&mut stripped, &mut e);
    let line = stripped.line;
    ENode {
        kind: EKind::QuestionE(Box::new(strip_q(stripped))),
        line,
    }
}

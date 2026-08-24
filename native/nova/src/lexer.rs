use crate::errors::{NovaError, Result};
use crate::messages;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokKind {
    Word,
    Str,
    Number,
    Newline,
    Eof,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Lt,
    Gt,
    Bang,
    Dot,
    LBrace,
    RBrace,
    Question,
    EqualEqual,
    BangEqual,
    Lte,
    Gte,
    AmpAmp,
    PipePipe,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumLit {
    Int(String),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokValue {
    Empty,
    Word(String),
    Text(String),
    Num(NumLit),
    Sym(&'static str),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokKind,
    pub value: TokValue,
    pub line: usize,
    pub col: usize,
}

impl Token {
    fn sym(kind: TokKind, sym: &'static str, line: usize, col: usize) -> Self {
        Token { kind, value: TokValue::Sym(sym), line, col }
    }

    pub fn word_str(&self) -> Option<&str> {
        match &self.value {
            TokValue::Word(w) => Some(w.as_str()),
            _ => None,
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self.kind {
            TokKind::Word => "WORD",
            TokKind::Str => "STRING",
            TokKind::Number => "NUMBER",
            TokKind::Newline => "NEWLINE",
            TokKind::Eof => "EOF",
            TokKind::LParen => "LPAREN",
            TokKind::RParen => "RPAREN",
            TokKind::LBracket => "LBRACKET",
            TokKind::RBracket => "RBRACKET",
            TokKind::Comma => "COMMA",
            TokKind::Equals => "EQUALS",
            TokKind::Plus => "PLUS",
            TokKind::Minus => "MINUS",
            TokKind::Star => "STAR",
            TokKind::Slash => "SLASH",
            TokKind::Percent => "PERCENT",
            TokKind::Lt => "LT",
            TokKind::Gt => "GT",
            TokKind::Bang => "BANG",
            TokKind::Dot => "DOT",
            TokKind::LBrace => "LBRACE",
            TokKind::RBrace => "RBRACE",
            TokKind::Question => "QUESTION",
            TokKind::EqualEqual => "EQUALEQUAL",
            TokKind::BangEqual => "BANGEQUAL",
            TokKind::Lte => "LTE",
            TokKind::Gte => "GTE",
            TokKind::AmpAmp => "AMPAMP",
            TokKind::PipePipe => "PIPEPIPE",
        };
        match &self.value {
            TokValue::Empty => write!(f, "{kind}('')"),
            TokValue::Word(w) => write!(f, "{kind}('{}')", py_repr(w)),
            TokValue::Text(t) => write!(f, "{kind}('{}')", py_repr(t)),
            TokValue::Num(NumLit::Int(s)) => write!(f, "{kind}({s})"),
            TokValue::Num(NumLit::Float(v)) => {
                if v.fract() == 0.0 && v.is_finite() && v.abs() < 1e16 {
                    write!(f, "{kind}({:.1})", v)
                } else {
                    write!(f, "{kind}({})", v)
                }
            }
            TokValue::Sym(s) => write!(f, "{kind}('{s}')"),
        }
    }
}

fn py_repr(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\'', "\\'")
}

const SKIN_DOUBLE: &[(&str, TokKind)] = &[
    ("==", TokKind::EqualEqual),
    ("!=", TokKind::BangEqual),
    ("<=", TokKind::Lte),
    (">=", TokKind::Gte),
    ("&&", TokKind::AmpAmp),
    ("||", TokKind::PipePipe),
];

fn skin_single(c: char) -> Option<TokKind> {
    Some(match c {
        '=' => TokKind::Equals,
        '+' => TokKind::Plus,
        '-' => TokKind::Minus,
        '*' => TokKind::Star,
        '/' => TokKind::Slash,
        '%' => TokKind::Percent,
        '<' => TokKind::Lt,
        '>' => TokKind::Gt,
        '!' => TokKind::Bang,
        '.' => TokKind::Dot,
        '{' => TokKind::LBrace,
        '}' => TokKind::RBrace,
        '?' => TokKind::Question,
        _ => return None,
    })
}

fn structural_single(c: char) -> Option<TokKind> {
    Some(match c {
        '(' => TokKind::LParen,
        ')' => TokKind::RParen,
        '[' => TokKind::LBracket,
        ']' => TokKind::RBracket,
        ',' => TokKind::Comma,
        _ => return None,
    })
}

const ESCAPES: &[(&str, &str)] = &[
    ("n", "\n"),
    ("t", "\t"),
    ("\\", "\\"),
    ("\"", "\""),
    ("'", "'"),
    ("{", "{"),
    ("}", "}"),
];

struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    bol: usize,
    _src: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        let src = src.strip_prefix('\u{feff}').unwrap_or(src);
        Lexer { chars: src.chars().collect(), pos: 0, line: 1, bol: 0, _src: src }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, k: usize) -> Option<char> {
        self.chars.get(self.pos + k).copied()
    }

    fn col(&self) -> usize {
        self.pos - self.bol + 1
    }

    fn push_symbol(&self, kind: TokKind, sym: &'static str, toks: &mut Vec<Token>) {
        toks.push(Token::sym(kind, sym, self.line, self.col()));
    }

    fn err(&self, msg: String) -> NovaError {
        NovaError::new(self.line, Some(self.col()), msg)
    }

    fn skip_line(&mut self) {
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.pos += 1;
        }
    }
}

pub fn lex(src: &str) -> Result<Vec<Token>> {
    let mut lx = Lexer::new(src);
    let mut toks = Vec::new();

    while let Some(c) = lx.peek() {
        if c == '\n' {
            toks.push(Token::sym(TokKind::Newline, "\n", lx.line, lx.col()));
            lx.line += 1;
            lx.pos += 1;
            lx.bol = lx.pos;
            continue;
        }
        if c == ' ' || c == '\t' || c == '\r' {
            lx.pos += 1;
            continue;
        }
        if lx.line == 1 && c == '#' && lx.peek_at(1) == Some('!') {
            lx.skip_line();
            continue;
        }
        if c == '#' || (c == '/' && lx.peek_at(1) == Some('/')) {
            lx.skip_line();
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            let c0 = lx.col();
            lx.pos += 1;
            let mut buf = String::new();
            loop {
                let ch = match lx.peek() {
                    Some(ch) => ch,
                    None => return Err(lx.err(messages::lex::unterminated())),
                };
                if ch == '\\' && lx.peek_at(1).is_some() {
                    let nxt = lx.peek_at(1).unwrap().to_string();
                    let rep = ESCAPES.iter().find(|(k, _)| *k == nxt).map(|(_, v)| *v);
                    match rep {
                        Some(rep) => buf.push_str(rep),
                        None => return Err(lx.err(messages::lex::bad_escape(&nxt))),
                    }
                    lx.pos += 2;
                    continue;
                }
                if ch == quote {
                    lx.pos += 1;
                    break;
                }
                if ch == '\n' {
                    return Err(lx.err(messages::lex::newline_in_string()));
                }
                buf.push(ch);
                lx.pos += 1;
            }
            toks.push(Token { kind: TokKind::Str, value: TokValue::Text(buf), line: lx.line, col: c0 });
            continue;
        }
        if c.is_ascii_digit() {
            let c0 = lx.col();
            let mut j = lx.pos;
            while let Some(ch) = lx.chars.get(j) {
                if ch.is_ascii_digit() || *ch == '_' {
                    j += 1;
                } else {
                    break;
                }
            }
            let is_float = lx.chars.get(j) == Some(&'.')
                && lx.chars.get(j + 1).is_some_and(|ch| ch.is_ascii_digit());
            let raw: String = lx.chars[lx.pos..j].iter().collect();
            if is_float {
                let mut frac = String::new();
                frac.push('.');
                j += 1;
                while let Some(ch) = lx.chars.get(j) {
                    if ch.is_ascii_digit() || *ch == '_' {
                        frac.push(*ch);
                        j += 1;
                    } else {
                        break;
                    }
                }
                let full = format!("{raw}{frac}");
                let val: f64 = full.replace('_', "").parse().map_err(|_| {
                    lx.err(format!("invalid number '{full}'"))
                })?;
                toks.push(Token {
                    kind: TokKind::Number,
                    value: TokValue::Num(NumLit::Float(val)),
                    line: lx.line,
                    col: c0,
                });
            } else {
                let cleaned = raw.replace('_', "");
                let trimmed = cleaned.trim_start_matches('0');
                let canon = if trimmed.is_empty() { "0" } else { trimmed };
                toks.push(Token {
                    kind: TokKind::Number,
                    value: TokValue::Num(NumLit::Int(canon.to_string())),
                    line: lx.line,
                    col: c0,
                });
            }
            lx.pos = j;
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let c0 = lx.col();
            let mut j = lx.pos;
            while let Some(&ch) = lx.chars.get(j) {
                if ch.is_alphanumeric() || ch == '_' {
                    j += 1;
                } else if ch == '-' {
                    match lx.chars.get(j + 1) {
                        Some(next) if next.is_ascii_digit() => break,
                        _ => j += 1,
                    }
                } else {
                    break;
                }
            }
            let mut word: String = lx.chars[lx.pos..j].iter().collect();
            while word.ends_with('-') {
                word.pop();
            }
            toks.push(Token { kind: TokKind::Word, value: TokValue::Word(word), line: lx.line, col: c0 });
            lx.pos = j;
            continue;
        }
        let two: String = lx.chars[lx.pos..(lx.pos + 2).min(lx.chars.len())].iter().collect();
        if let Some((sym, kind)) = SKIN_DOUBLE.iter().find(|(s, _)| *s == two) {
            lx.push_symbol(*kind, sym, &mut toks);
            lx.pos += 2;
            continue;
        }
        if let Some(kind) = skin_single(c) {
            let sym: &'static str = match kind {
                TokKind::Equals => "=",
                TokKind::Plus => "+",
                TokKind::Minus => "-",
                TokKind::Star => "*",
                TokKind::Slash => "/",
                TokKind::Percent => "%",
                TokKind::Lt => "<",
                TokKind::Gt => ">",
                TokKind::Bang => "!",
                TokKind::Dot => ".",
                TokKind::LBrace => "{",
                TokKind::RBrace => "}",
                TokKind::Question => "?",
                _ => unreachable!(),
            };
            lx.push_symbol(kind, sym, &mut toks);
            lx.pos += 1;
            continue;
        }
        if let Some(kind) = structural_single(c) {
            let sym: &'static str = match kind {
                TokKind::LParen => "(",
                TokKind::RParen => ")",
                TokKind::LBracket => "[",
                TokKind::RBracket => "]",
                TokKind::Comma => ",",
                _ => unreachable!(),
            };
            lx.push_symbol(kind, sym, &mut toks);
            lx.pos += 1;
            continue;
        }
        if c == ';' {
            lx.push_symbol(TokKind::Newline, ";", &mut toks);
            lx.pos += 1;
            continue;
        }
        return Err(lx.err(messages::lex::bad_char(&c.to_string())));
    }
    toks.push(Token::sym(TokKind::Newline, "\n", lx.line, lx.col()));
    toks.push(Token::sym(TokKind::Eof, "", lx.line, lx.col()));
    Ok(toks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<(TokKind, String)> {
        lex(src)
            .unwrap()
            .into_iter()
            .filter(|t| t.kind != TokKind::Eof)
            .map(|t| {
                let v = match t.value {
                    TokValue::Word(w) => w,
                    TokValue::Text(s) => s,
                    TokValue::Num(NumLit::Int(i)) => i,
                    TokValue::Num(NumLit::Float(f)) => format!("{f}"),
                    TokValue::Sym(s) => s.to_string(),
                    TokValue::Empty => String::new(),
                };
                (t.kind, v)
            })
            .collect()
    }

    #[test]
    fn words_numbers_symbols_basic_sentence() {
        let ks = kinds("say \"hi\" plus 42");
        assert_eq!(
            ks,
            vec![
                (TokKind::Word, "say".into()),
                (TokKind::Str, "hi".into()),
                (TokKind::Word, "plus".into()),
                (TokKind::Number, "42".into()),
                (TokKind::Newline, "\n".into()),
            ]
        );
    }

    #[test]
    fn hyphen_policy_word_vs_minus() {
        let ks = kinds("guess-count x-1 end-");
        assert_eq!(ks[0], (TokKind::Word, "guess-count".into()));
        assert_eq!(ks[1], (TokKind::Word, "x".into()));
        assert_eq!(ks[2], (TokKind::Minus, "-".into()));
        assert_eq!(ks[3], (TokKind::Number, "1".into()));
        assert_eq!(ks[4], (TokKind::Word, "end".into()));
    }

    #[test]
    fn numbers_underscores_float_and_trailing_dot() {
        let ks = kinds("1_000 3.5 1.");
        assert_eq!(ks[0], (TokKind::Number, "1000".into()));
        assert_eq!(ks[1].0, TokKind::Number);
        assert!(matches!(kinds("3.5")[0], _));
        assert_eq!(ks[2], (TokKind::Number, "1".into()));
        assert_eq!(ks[3], (TokKind::Dot, ".".into()));
    }

    #[test]
    fn leading_zero_integers_canonicalize_like_python() {
        assert_eq!(kinds("007")[0], (TokKind::Number, "7".into()));
        assert_eq!(kinds("000")[0], (TokKind::Number, "0".into()));
    }

    #[test]
    fn string_escapes_including_braces() {
        let ks = kinds(r#""a\nb" 'it\'s \{ \}'"#);
        assert_eq!(ks[0], (TokKind::Str, "a\nb".into()));
        assert_eq!(ks[1], (TokKind::Str, "it's { }".into()));
    }

    #[test]
    fn skin_doubles_before_singles() {
        let ks = kinds("a == b != c <= d >= e && f || g = h");
        let kinds_only: Vec<TokKind> = ks.iter().map(|(k, _)| *k).collect();
        assert!(kinds_only.contains(&TokKind::EqualEqual));
        assert!(kinds_only.contains(&TokKind::BangEqual));
        assert!(kinds_only.contains(&TokKind::Lte));
        assert!(kinds_only.contains(&TokKind::Gte));
        assert!(kinds_only.contains(&TokKind::AmpAmp));
        assert!(kinds_only.contains(&TokKind::PipePipe));
        assert!(kinds_only.contains(&TokKind::Equals));
    }

    #[test]
    fn semicolon_is_a_newline_token() {
        let ks = kinds("say 1; say 2");
        assert_eq!(ks[2], (TokKind::Newline, ";".into()));
    }

    #[test]
    fn shebang_bom_and_comments_skipped() {
        let src = "\u{feff}#!/usr/bin/env nova\n# comment\nsay 1 // trailing\n";
        let ks: Vec<_> = kinds(src).into_iter().filter(|(k, _)| *k != TokKind::Newline).collect();
        assert_eq!(
            ks,
            vec![
                (TokKind::Word, "say".into()),
                (TokKind::Number, "1".into()),
            ]
        );
    }

    #[test]
    fn unicode_words_survive() {
        assert_eq!(kinds("héllo")[0], (TokKind::Word, "héllo".into()));
    }

    #[test]
    fn structural_punct() {
        let ks = kinds("[1, (2)] ?");
        let kinds_only: Vec<TokKind> = ks.iter().map(|(k, _)| *k).collect();
        assert_eq!(
            kinds_only,
            vec![
                TokKind::LBracket,
                TokKind::Number,
                TokKind::Comma,
                TokKind::LParen,
                TokKind::Number,
                TokKind::RParen,
                TokKind::RBracket,
                TokKind::Question,
                TokKind::Newline,
            ]
        );
    }

    #[test]
    fn error_unterminated_string() {
        let e = lex("\"abc").unwrap_err();
        assert_eq!(e.msg, "unterminated string — add the missing quote");
        assert_eq!(e.to_string(), "line 1: unterminated string — add the missing quote");
    }

    #[test]
    fn error_newline_in_string() {
        let e = lex("\"ab\nc\"").unwrap_err();
        assert_eq!(e.msg, r"newline inside a string — use \n or split the text");
    }

    #[test]
    fn error_bad_escape_matches_oracle_bytes() {
        let e = lex(r#""bad \q""#).unwrap_err();
        assert_eq!(
            e.msg,
            "invalid escape '\\q' — valid: \\\\n \\\\t \\\\\\\\ \\\" ' { }"
        );
    }

    #[test]
    fn error_bad_char() {
        let e = lex("x @ y").unwrap_err();
        assert_eq!(e.col, Some(3));
        assert!(e.msg.starts_with("the character '@' is not valid"));
    }

    #[test]
    fn columns_are_one_based_per_line() {
        let toks = lex("say hi\n  bye").unwrap();
        assert_eq!(toks[0].col, 1);
        let bye = toks.iter().find(|t| t.word_str() == Some("bye")).unwrap();
        assert_eq!(bye.col, 3);
        assert_eq!(bye.line, 2);
    }
}

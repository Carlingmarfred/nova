//! User-facing diagnostics — single source of truth, ported from
//! `bootstrap/nova_messages.py`. Sentences must stay byte-identical to the
//! Python oracle so the differential harness can compare stderr verbatim.

fn interpolate(template: &str, args: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, val) in args {
        let mut pat = String::from("{");
        pat.push_str(key);
        pat.push('}');
        if out.contains(&pat) {
            out = out.replace(&pat, val);
        } else {
            panic!("message placeholder {pat} not in template: {template}");
        }
    }
    out.replace("{{", "{").replace("}}", "}")
}

pub fn suggest(near: &str) -> String {
    interpolate("— did you mean '{near}'?", &[("near", near)])
}

pub fn reserved(word: &str, what: &str) -> String {
    interpolate(
        "'{word}' is a reserved word and cannot be used as a {what} — choose a different name",
        &[("word", word), ("what", what)],
    )
}

pub fn expected_name(what: &str, found: &str) -> String {
    interpolate(
        "expected a {what}, found '{found}'",
        &[("what", what), ("found", found)],
    )
}

pub mod lex {
    use super::interpolate;

    pub fn unterminated() -> String {
        "unterminated string — add the missing quote".to_string()
    }

    pub fn bad_escape(ch: &str) -> String {
        interpolate(
            r#"invalid escape '\{ch}' — valid: \\n \\t \\\\ \" ' {{ }}"#,
            &[("ch", ch)],
        )
    }

    pub fn newline_in_string() -> String {
        r"newline inside a string — use \n or split the text".to_string()
    }

    pub fn bad_char(ch: &str) -> String {
        interpolate(
            "the character '{ch}' is not valid in Nova Natural — \
             check for typos; variables cannot contain special characters",
            &[("ch", ch)],
        )
    }
}

pub mod parse {
    use super::interpolate;

    pub fn expected_word(wanted: &str, found: &str) -> String {
        interpolate(
            "expected '{wanted}' but found '{found}' — check the wording of the sentence",
            &[("wanted", wanted), ("found", found)],
        )
    }

    pub fn expected_eol(found: &str) -> String {
        interpolate(
            "unexpected '{found}' — expected end of line (one sentence per line)",
            &[("found", found)],
        )
    }

    pub fn unexpected_start(found: &str) -> String {
        "unexpected '{found}' — expected a sentence (e.g.: say ... / set ... to ... / repeat ...) or a declaration like 'x is 5'"
            .replace("{found}", found)
    }

    pub fn block_unclosed(stops: &str) -> String {
        interpolate(
            "the block is never closed — expected '{stops}'; every block ends with 'done'",
            &[("stops", stops)],
        )
    }

    pub fn block_missing_done() -> String {
        "the block is missing 'done' — every block ends with its own done".to_string()
    }

    pub fn check_missing_done() -> String {
        "check is missing 'done' — close the arm list with done".to_string()
    }

    pub fn missing_rparen() -> String {
        "missing ')'".to_string()
    }

    pub fn list_sep() -> String {
        "expected ',' or ']' in the list".to_string()
    }

    pub fn unexpected_in_expr(found: &str) -> String {
        interpolate(
            "unexpected '{found}' in an expression",
            &[("found", found)],
        )
    }

    pub fn unknown_time_unit(unit: &str) -> String {
        interpolate(
            "unknown time unit '{unit}' (use seconds/minutes/hours/milliseconds)",
            &[("unit", unit)],
        )
    }

    pub fn set_the_form() -> String {
        "'set' with 'the' requires the form: the <field> of <object> (e.g.: set the text of task to \"hello\")"
            .to_string()
    }

    pub fn expected_name_after_the(found: &str) -> String {
        interpolate(
            "expected a name after 'the', found '{found}' — e.g.: the text of task",
            &[("found", found)],
        )
    }

    pub fn method_call_later() -> String {
        "method calls like .name(...) arrive in a later version — in this version dot-calls are only for module functions: module-name.function(...)"
            .to_string()
    }

    pub fn module_name_rule(found: &str) -> String {
        interpolate(
            "a module name must end in '-module' — e.g.: the tools-module in \"tools.nova\" (found '{found}')",
            &[("found", found)],
        )
    }

    pub fn module_path_expected(name: &str) -> String {
        interpolate(
            "expected a quoted file path after 'in' — e.g.: the {name} in \"tools.nova\"",
            &[("name", name)],
        )
    }

    pub fn lvalue_name(found: &str) -> String {
        interpolate(
            "expected a name, found '{found}'",
            &[("found", found)],
        )
    }
}

pub mod interp {
    use super::interpolate;

    pub fn condition_not_bool() -> String {
        "a condition must be true or false (use comparisons like 'is greater than')".to_string()
    }

    pub fn ordering_needs_numbers(found: &str) -> String {
        interpolate(
            "ordering needs two numbers — found {found} — use 'is' or 'is not' to compare other values",
            &[("found", found)],
        )
    }

    pub fn plus_type_mismatch(left: &str, right: &str) -> String {
        interpolate(
            "cannot add {left} and {right} — '+' requires two numbers or two texts",
            &[("left", left), ("right", right)],
        )
    }

    pub fn div_by_zero() -> String {
        "division by zero — check the denominator, or use 'if x is 0' first".to_string()
    }

    pub fn mod_by_zero() -> String {
        "modulo by zero — check the divisor, or use 'if x is 0' first".to_string()
    }

    pub fn arith_on_nothing() -> String {
        "cannot do arithmetic on 'nothing' — add '?' if the expression may be nothing (e.g.: n = the number value of answer? + 1), or check the value with 'is nothing' first"
            .to_string()
    }

    pub fn contains_needs_str_or_list() -> String {
        "'contains' requires text or a list".to_string()
    }

    pub fn var_not_found(name: &str) -> String {
        format!(
            "the variable '{name}' does not exist — check the spelling or declare it with 'x is ...'"
        )
    }

    pub fn each_needs_seq(_found: &str) -> String {
        "'repeat for each' requires a list or text".to_string()
    }

    pub fn times_needs_num(_found: &str) -> String {
        "'repeat N times' requires a number".to_string()
    }

    pub fn counting_needs_num(_found: &str) -> String {
        "'repeat with i from A to B' requires numbers".to_string()
    }

    pub fn add_needs_list_or_num(name: &str) -> String {
        format!("'add ... to {name}' requires a list or a number")
    }

    pub fn func_not_found(name: &str) -> String {
        format!(
            "the function '{name}' does not exist — define it with 'to <name> ... done'"
        )
    }

    pub fn func_arity(name: &str, wanted: usize, got: usize, call_hint: &str) -> String {
        format!("'{name}' expects {wanted} argument(s), got {got} — call: {call_hint}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_fragments_match_oracle() {
        assert_eq!(suggest("nam"), "— did you mean 'nam'?");
        assert_eq!(
            reserved("set", "variable name"),
            "'set' is a reserved word and cannot be used as a variable name — choose a different name"
        );
        assert_eq!(expected_name("field name", "5"), "expected a field name, found '5'");
    }

    #[test]
    fn lexer_messages_match_oracle_byte_for_byte() {
        assert_eq!(lex::unterminated(), "unterminated string — add the missing quote");
        assert_eq!(
            lex::bad_escape("q"),
            "invalid escape '\\q' — valid: \\\\n \\\\t \\\\\\\\ \\\" ' { }"
        );
        assert_eq!(
            lex::newline_in_string(),
            r"newline inside a string — use \n or split the text"
        );
        assert_eq!(
            lex::bad_char("@"),
            "the character '@' is not valid in Nova Natural — check for typos; variables cannot contain special characters"
        );
    }

    #[test]
    fn parser_messages_match_oracle() {
        assert_eq!(
            parse::expected_word("done", "odn"),
            "expected 'done' but found 'odn' — check the wording of the sentence"
        );
        assert_eq!(
            parse::block_unclosed("'done'"),
            "the block is never closed — expected ''done''; every block ends with 'done'"
        );
        assert_eq!(
            parse::module_name_rule("tools"),
            "a module name must end in '-module' — e.g.: the tools-module in \"tools.nova\" (found 'tools')"
        );
        assert_eq!(
            parse::module_path_expected("the tools-module"),
            "expected a quoted file path after 'in' — e.g.: the the tools-module in \"tools.nova\""
        );
    }

    #[test]
    fn interp_messages_match_oracle() {
        assert_eq!(
            interp::plus_type_mismatch("true", "1"),
            "cannot add true and 1 — '+' requires two numbers or two texts"
        );
        assert_eq!(
            interp::condition_not_bool(),
            "a condition must be true or false (use comparisons like 'is greater than')"
        );
    }

    #[test]
    #[should_panic]
    fn unknown_placeholder_panics_loudly() {
        interpolate("no hole here", &[("x", "1")]);
    }
}

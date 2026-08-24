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

    pub fn contract_failed(kind: &str) -> String {
        format!("the {kind} contract failed — the condition on this line was not true")
    }

    pub fn ensure_failed(func: &str) -> String {
        format!(
            "the ensures contract failed in '{func}' — the final state did not satisfy the guarantee"
        )
    }

    pub fn thing_missing_field(cls: &str, name: &str, valid: &str) -> String {
        format!("{cls} has no field '{name}' — valid fields: {valid}")
    }

    pub fn field_of_nothing(name: &str) -> String {
        format!(
            "cannot read the field '{name}' of nothing — add '?' if the expression may be nothing (e.g.: the {name} of x?), or check the value with 'is nothing' first"
        )
    }

    pub fn cannot_read_field(name: &str, value: &str) -> String {
        format!("cannot read the field '{name}' of {value}")
    }

    pub fn field_needs_thing() -> String {
        "you can only set fields on a thing".to_string()
    }

    pub fn unknown_thing(name: &str) -> String {
        format!("unknown thing '{name}'")
    }

    pub fn no_changes(name: &str, op: &str) -> String {
        format!("there are no changes to {op} for '{name}'")
    }

    pub fn item_out_of_bounds(idx: i64, size: i64) -> String {
        format!("item {idx} does not exist (there are {size} items) — valid numbers are 1 to {size}")
    }

    pub fn item_needs_list() -> String {
        "'item N of' requires a list".to_string()
    }

    pub fn item_needs_num_index() -> String {
        "'item N of' requires a number as the index".to_string()
    }

    pub fn text_at_oob(idx: i64, size: i64) -> String {
        format!("position {idx} does not exist (the text has only {size} characters) — valid positions are 1 to {size}")
    }

    pub fn count_needs_sized() -> String {
        "'how many items are in' requires a list or text".to_string()
    }

    pub fn random_needs_nums() -> String {
        "'a random number between A and B' requires numbers".to_string()
    }
}


pub mod modules {
    use super::interpolate;

    pub fn module_file_not_found(path: &str, dir: &str) -> String {
        interpolate(
            "module file '{path}' not found (searched in '{dir}') \u{2014} check the path and the filename",
            &[("path", path), ("dir", dir)],
        )
    }

    pub fn circular_import(chain: &str) -> String {
        interpolate(
            "circular import: {chain} \u{2014} modules cannot import each other in a ring; break the chain by moving what they share into a third file",
            &[("chain", chain)],
        )
    }

    pub fn no_mains() -> String {
        "a module must not contain 'when the program starts' \u{2014} move the program start to the main program".to_string()
    }

    pub fn not_a_module(name: &str) -> String {
        interpolate(
            "'{name}' is not a module \u{2014} dot-calls require 'the {name}-module in \"file.nova\"' first",
            &[("name", name)],
        )
    }

    pub fn module_no_function(path: &str, name: &str, hint: &str, call: &str) -> String {
        interpolate(
            "module '{path}' has no function '{name}'{hint} \u{2014} call: {call}",
            &[("path", path), ("name", name), ("hint", hint), ("call", call)],
        )
    }

    pub fn module_no_member(path: &str, name: &str, hint: &str, names: &str) -> String {
        interpolate(
            "module '{path}' has no '{name}'{hint} \u{2014} valid names: {names}",
            &[("path", path), ("name", name), ("hint", hint), ("names", names)],
        )
    }
}

pub mod stdlib {
    use super::interpolate;

    pub fn use_form(text: &str) -> String {
        interpolate(
            "unknown 'use' form: '{text}' \u{2014} write: use the standard <name> library",
            &[("text", text)],
        )
    }

    pub fn unknown_lib(name: &str, libs: &str) -> String {
        interpolate(
            "unknown standard library '{name}' \u{2014} available libraries: {libs}",
            &[("name", name), ("libs", libs)],
        )
    }

    pub fn json_parse_needs_text() -> String {
        "'json.parse' requires text \u{2014} give it a string containing json".to_string()
    }

    pub fn json_parse_invalid(line: usize) -> String {
        interpolate(
            "invalid json (line {line}) \u{2014} check the text, or catch the failure with 'try ... if it fails'",
            &[("line", &line.to_string())],
        )
    }

    pub fn file_write_needs_text() -> String {
        "'file.write' requires text as the content".to_string()
    }

    pub fn missing_file(path: &str) -> String {
        interpolate("'{path}' does not exist", &[("path", path)])
    }

    pub fn is_directory(path: &str) -> String {
        interpolate("'{path}' is a directory, not a file", &[("path", path)])
    }

    pub fn not_utf8(path: &str) -> String {
        interpolate("'{path}' is not a UTF-8 text file", &[("path", path)])
    }

    pub fn cannot_read(path: &str, err: &str) -> String {
        interpolate("cannot read '{path}': {err}", &[("path", path), ("err", err)])
    }

    pub fn cannot_save(path: &str, err: &str) -> String {
        interpolate("cannot save to '{path}': {err}", &[("path", path), ("err", err)])
    }

    pub fn random_between_needs_nums() -> String {
        "'random.between' requires two numbers".to_string()
    }

    pub fn random_pick_needs_list() -> String {
        "'random.pick' requires a non-empty list".to_string()
    }

    pub fn random_shuffle_needs_list(value: &str) -> String {
        interpolate("'random.shuffle' requires a list \u{2014} found {value}", &[("value", value)])
    }

    pub fn time_sleep_needs_num() -> String {
        "'time.sleep' requires a number (seconds)".to_string()
    }

    pub fn time_sleep_negative() -> String {
        "'time.sleep' cannot sleep for a negative number of seconds".to_string()
    }

    pub fn math_sqrt_negative() -> String {
        "'math.sqrt' requires a number that is 0 or greater".to_string()
    }

    pub fn math_needs_num(fn_name: &str) -> String {
        interpolate("'math.{fn}' requires a number", &[("fn", fn_name)])
    }

    pub fn math_pow_needs_nums() -> String {
        "'math.pow' requires two numbers (base and exponent)".to_string()
    }

    pub fn text_needs_text(fn_name: &str, value: &str) -> String {
        interpolate(
            "'text.{fn}' requires text \u{2014} found {value}",
            &[("fn", fn_name), ("value", value)],
        )
    }

    pub fn text_split_empty_sep() -> String {
        "'text.split' requires a non-empty separator".to_string()
    }

    pub fn text_join_needs_list() -> String {
        "'text.join' requires a list \u{2014} give it text.split(...) output first".to_string()
    }

    pub fn text_replace_empty_search() -> String {
        "'text.replace' requires a non-empty search text".to_string()
    }

    pub fn text_at_needs_num() -> String {
        "'text.at' requires a number as the position".to_string()
    }

    pub fn text_at_out_of_bounds(idx: i64, size: i64) -> String {
        let max = size.max(1).to_string();
        interpolate(
            "position {idx} does not exist (the text has only {size} characters) \u{2014} valid positions are 1 to {max}",
            &[("idx", &idx.to_string()), ("size", &size.to_string()), ("max", &max)],
        )
    }

    pub fn text_slice_needs_nums() -> String {
        "'text.slice' requires numbers for start/end (1-based, inclusive)".to_string()
    }

    pub fn text_slice_out_of_bounds(start: i64, end: i64, size: i64) -> String {
        let max = size.max(1).to_string();
        interpolate(
            "slice {start} to {end} reaches outside the text \u{2014} valid end values are 1 to {max}",
            &[("start", &start.to_string()), ("end", &end.to_string()), ("max", &max)],
        )
    }

    pub fn list_needs_list(fn_name: &str, value: &str) -> String {
        interpolate(
            "'list.{fn}' requires a list \u{2014} found {value}",
            &[("fn", fn_name), ("value", value)],
        )
    }

    pub fn list_sort_mixed(types: &str) -> String {
        interpolate(
            "'list.sort' cannot mix types ({types}) \u{2014} give it a list of EITHER numbers OR text",
            &[("types", types)],
        )
    }

    pub fn list_min_max_empty(fn_name: &str) -> String {
        interpolate("'list.{fn}' requires a non-empty list", &[("fn", fn_name)])
    }

    pub fn list_min_max_needs_nums(fn_name: &str) -> String {
        interpolate("'list.{fn}' requires a list of numbers", &[("fn", fn_name)])
    }

    pub fn list_keys_needs_dict(value: &str) -> String {
        interpolate(
            "'list.keys' requires a dictionary (e.g. from json.parse) \u{2014} found {value}",
            &[("value", value)],
        )
    }

    pub fn list_values_needs_dict(value: &str) -> String {
        interpolate(
            "'list.values' requires a dictionary (e.g. from json.parse) \u{2014} found {value}",
            &[("value", value)],
        )
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

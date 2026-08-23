"""Nova bootstrap user-facing messages (single source of truth).

All runtime diagnostics live here so future localization is a pure data task.
Placeholders use str.format(). Style law (ITERATION_PLAN §1 rule 3): every
message is a sentence that says what happened and how to fix it.
"""

M = {
    # ---- shared fragments ----
    "suggest": "— did you mean '{near}'?",
    "reserved": "'{word}' is a reserved word and cannot be used as a {what} — choose a different name",
    "expected_name": "expected a {what}, found '{found}'",

    # ---- lexer ----
    "lex.unterminated": "unterminated string — add the missing quote",
    "lex.bad_escape": r"invalid escape '\{ch}' — valid: \\n \\t \\\\ \" ' {{ }}",
    "lex.newline_in_string": r"newline inside a string — use \n or split the text",
    "lex.bad_char": "the character '{ch}' is not valid in Nova Natural — "
                    "check for typos; variables cannot contain special characters",

    # ---- parser ----
    "parse.expected_word": "expected '{wanted}' but found '{found}' — check the wording of the sentence",
    "parse.expected_eol": "unexpected '{found}' — expected end of line (one sentence per line)",
    "parse.unexpected_start": "unexpected '{found}' — expected a sentence (e.g.: say ... / set ... to ... / repeat ...) "
                              "or a declaration like 'x is 5'",
    "parse.block_unclosed": "the block is never closed — expected '{stops}'; every block ends with 'done'",
    "parse.block_missing_done": "the block is missing 'done' — every block ends with its own done",
    "parse.check_missing_done": "check is missing 'done' — close the arm list with done",
    "parse.missing_rparen": "missing ')'",
    "parse.list_sep": "expected ',' or ']' in the list",
    "parse.unexpected_in_expr": "unexpected '{found}' in an expression",
    "parse.unknown_time_unit": "unknown time unit '{unit}' (use seconds/minutes/hours/milliseconds)",
    "parse.set_the_form": "'set' with 'the' requires the form: the <field> of <object> "
                          "(e.g.: set the text of task to \"hello\")",
    "parse.expected_name_after_the": "expected a name after 'the', found '{found}' — e.g.: the text of task",
    "parse.method_call_later": "method calls like .name(...) arrive in a later version — in this version "
                               "dot-calls are only for module functions: module-name.function(...)",
    "parse.module_name_rule": "a module name must end in '-module' — e.g.: the tools-module in \"tools.nova\" (found '{found}')",
    "parse.module_path_expected": "expected a quoted file path after 'in' — e.g.: the {name} in \"tools.nova\"",
    "parse.lvalue_name": "expected a name, found '{found}'",

    # ---- interpreter: values & scope ----
    "var.not_found": "the variable '{name}' does not exist{hint} — check the spelling or declare it with 'x is ...'",
    "assign.field_needs_thing": "you can only set fields on a thing",
    "assign.invalid_target": "invalid assignment target",
    "add.needs_list_or_num": "'add ... to {name}' requires a list or a number",
    "take.needs_list_or_num": "'take ... from {name}' requires a list or a number",

    # ---- interpreter: control flow ----
    "repeat.times_needs_num": "'repeat N times' requires a number",
    "repeat.each_needs_seq": "'repeat for each' requires a list or text",
    "repeat.counting_needs_num": "'repeat with i from A to B' requires numbers",
    "contract.failed": "the {kind} contract failed — the condition on this line was not true",
    "contract.ensure_failed": "the ensures contract failed in '{name}' — the final state did not satisfy the guarantee",
    "condition.not_bool": "a condition must be true or false (use comparisons like 'is greater than')",

    # ---- interpreter: collections ----
    "remove.only_item": "'remove' supports: remove item N of LIST",
    "remove.needs_list_num": "'remove item N of LIST' requires a list and a number",
    "remove.out_of_bounds": "item {idx} does not exist (the list has {size} items)",
    "item.of_needs_list": "'item N of' requires a list",
    "item.needs_num_index": "'item N of' requires a number as the index",
    "item.out_of_bounds": "item {idx} does not exist (there are {size} items) — valid numbers are 1 to {max}",
    "count.needs_sized": "'how many items are in' requires a list or text",
    "contains.needs_str_or_list": "'contains' requires text or a list",

    # ---- interpreter: fields & things ----
    "field.dict_missing_key": "the dictionary does not have the key '{name}'",
    "field.of_nothing": "cannot read the field '{name}' of nothing — add '?' if the expression may be nothing "
                        "(e.g.: the {name} of x?), or check the value with 'is nothing' first",
    "field.cannot_read": "cannot read the field '{name}' of {value}",
    "field.thing_missing": "{cls} has no field '{name}'{hint} — valid fields: {fields}",
    "thing.unknown": "unknown thing '{name}'",

    # ---- interpreter: arithmetic ----
    "arith.on_nothing": "cannot do arithmetic on 'nothing' — add '?' if the expression may be nothing "
                        "(e.g.: n = the number value of answer? + 1), or check the value with 'is nothing' first",
    "plus.type_mismatch": "cannot add {left} and {right} — '+' requires two numbers or two texts",
    "div.by_zero": "division by zero — check the denominator, or use 'if x is 0' first",
    "mod.by_zero": "modulo by zero — check the divisor, or use 'if x is 0' first",
    "random.needs_nums": "'a random number between A and B' requires numbers",

    # ---- interpreter: io ----
    "io.missing_file": "'{path}' does not exist",
    "io.is_directory": "'{path}' is a directory, not a file",
    "io.not_utf8": "'{path}' is not a UTF-8 text file",
    "io.bad_json": "'{path}' contains invalid json (line {line})",
    "io.cannot_read": "cannot read '{path}': {err}",
    "io.cannot_save": "cannot save to '{path}': {err}",
    "json.parse_needs_text": "'json.parse' requires text — give it a string containing json",
    "json.parse_invalid": "invalid json (line {line}) — check the text, or catch the failure with 'try ... if it fails'",
    "file.write_needs_text": "'file.write' requires text as the content",

    # ---- interpreter: functions & modules ----
    "func.not_found": "the function '{name}' does not exist{hint} — define it with 'to <name> ... done'",
    "func.arity": "'{name}' expects {wanted} argument(s), got {got} — call: {name} with {params}",
    "module.not_a_module": "'{name}' is not a module — dot-calls require 'the {name}-module in \"file.nova\"' first",
    "module.no_function": "module '{path}' has no function '{name}'{hint} — call: {mod}.{name}(...)",
    "module.no_member": "module '{path}' has no '{name}'{hint} — valid names: {names}",
    "module.circular_import": "circular import: {chain} — modules cannot import each other in a ring; "
                              "break the chain by moving what they share into a third file",
    "module.file_not_found": "module file '{path}' not found (searched in '{dir}') — check the path and the filename",
    "module.no_mains": "a module must not contain 'when the program starts' — move the program start to the main program",

    # ---- interpreter: stdlib ----
    "stdlib.use_form": "unknown 'use' form: '{text}' — write: use the standard <name> library",
    "stdlib.unknown_lib": "unknown standard library '{name}' — available libraries: {libs}",
    "random.between_needs_nums": "'random.between' requires two numbers",
    "random.pick_needs_list": "'random.pick' requires a non-empty list",
    "random.shuffle_needs_list": "'random.shuffle' requires a list — found {value}",
    "time.sleep_needs_num": "'time.sleep' requires a number (seconds)",
    "time.sleep_negative": "'time.sleep' cannot sleep for a negative number of seconds",
    "math.sqrt_negative": "'math.sqrt' requires a number that is 0 or greater",
    "math.needs_num": "'math.{fn}' requires a number",
    "math.pow_needs_nums": "'math.pow' requires two numbers (base and exponent)",
    "text.needs_text": "'text.{fn}' requires text — found {value}",
    "text.split_empty_sep": "'text.split' requires a non-empty separator",
    "text.join_needs_list": "'text.join' requires a list — give it text.split(...) output first",
    "text.replace_empty_search": "'text.replace' requires a non-empty search text",
    "text.at_out_of_bounds": "position {idx} does not exist (the text has only {size} characters)"
                             " — valid positions are 1 to {max}",
    "text.at_needs_num": "'text.at' requires a number as the position",
    "text.slice_out_of_bounds": "slice {start} to {end} reaches outside the text"
                                " — valid end values are 1 to {max}",
    "text.slice_needs_nums": "'text.slice' requires numbers for start/end (1-based, inclusive)",
    "list.needs_list": "'list.{fn}' requires a list — found {value}",
    "list.sort_mixed_types": "'list.sort' cannot mix types ({types}) — give it a list of EITHER numbers OR text",
    "list.min_max_empty": "'list.{fn}' requires a non-empty list",
    "list.min_max_needs_nums": "'list.{fn}' requires a list of numbers",
    "list.keys_needs_dict": "'list.keys' requires a dictionary (e.g. from json.parse) — found {value}",
    "list.values_needs_dict": "'list.values' requires a dictionary (e.g. from json.parse) — found {value}",

    # ---- interpreter: memory ----
    "copy.not_a_value": "'a copy of' cannot copy {value} — a module/function is not a value",

    # ---- interpreter: misc ----
    "unknown.statement": "unknown statement {kind}",
    "unknown.expression": "unknown expression {kind}",
}


def msg(key, **kw):
    return M[key].format(**kw)

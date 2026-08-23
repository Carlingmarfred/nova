"""End-to-end tests for the Nova bootstrap interpreter.

Kør:  python tests/run_tests.py
Alle tests kører CLI'en som subprocess (samme vej som brugeren).
"""

import json
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CLI = os.path.join(ROOT, "bootstrap", "nova_cli.py")
GOLDEN_DIR = os.path.join(ROOT, "tests", "golden")
ENV = dict(os.environ, PYTHONUTF8="1", PYTHONIOENCODING="utf-8")


def nova(args, stdin="", cwd=None, timeout=60):
    return subprocess.run(
        [sys.executable, CLI] + args,
        input=stdin, capture_output=True, text=True, encoding="utf-8",
        cwd=cwd or ROOT, env=ENV, timeout=timeout,
    )


_passed = 0
_failed = 0


def check(name, cond, info=""):
    global _passed, _failed
    if cond:
        _passed += 1
        print(f"OK    {name}")
    else:
        _failed += 1
        print(f"FAIL  {name}  {info}")


def prog(src, cwd):
    """Skriv en midlertidig .nova-fil og returnér stien."""
    fd, path = tempfile.mkstemp(suffix=".nova", dir=cwd)
    with os.fdopen(fd, "w", encoding="utf-8") as f:
        f.write(src)
    return path


# ---------------------------------------------------------------- CLI

def test_cli(tmp):
    p = nova(["version"])
    check("cli/version", p.returncode == 0 and "Nova" in p.stdout and "0.14" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r}")

    bad = os.path.join(tmp, "bad.nova")
    with open(bad, "w", encoding="utf-8") as f:
        f.write("say )(\n")
    p = nova(["run", bad])
    check("cli/syntax-error", p.returncode == 1 and "linje" in p.stderr,
          f"rc={p.returncode} err={p.stderr!r}")

    p = nova(["fisk"])
    check("cli/unknown-command", p.returncode == 2)

    p = nova(["run", os.path.join(ROOT, "examples", "guessing_game.nova"), "--seed", "abc"])
    check("cli/bad-seed", p.returncode == 2)

    missing = nova(["run", os.path.join(tmp, "findes_ikke.nova")])
    check("cli/missing-file", missing.returncode == 2)


# --------------------------------------------------- sprogsætninger

def test_phrases(tmp):
    cases = [
        # (navn, kilde, stdin, skal indeholde)
        ("interpolation", 'x is 5\nsay "x er {x}"', "", "x er 5"),
        ("random-between",
         'x is a random number between 5 and 5\nif x is 5 then say "ok"', "", "ok"),
        ("random-seeded", 'x is a random number between 1 and 10\nsay "{x}"', "", None),
        ("number-value",
         'answer is "42"\nn is the number value of answer\nsay "{n}"', "", "42"),
        ("is-not-a-number",
         'if "abc" is not a number then say "ja"', "", "ja"),
        ("length-of", 'xs is [1, 2, 3]\nsay "{the length of xs}"', "", "3"),
        ("first-last-item",
         'xs is [7, 8, 9]\nsay "{the first item of xs} {the last item of xs}"', "", "7 9"),
        ("item-n-of", 'xs is ["a", "b"]\nsay "{item 2 of xs}"', "", "b"),
        ("empty-list", 'xs is an empty list\nadd 5 to xs\nsay "{the first item of xs}"', "", "5"),
        ("repeat-times", 'repeat 3 times\n    say "hei"\ndone', "", "hei\nhei\nhei"),
        ("repeat-counting",
         'repeat with i from 1 to 3\n    say "{i}"\ndone', "", "1\n2\n3"),
        ("for-each", 'repeat for each x in [1, 2]\n    say "{x}"\ndone', "", "1\n2"),
        ("repeat-until-break",
         'n is 0\nrepeat until n is 3\n    add 1 to n\ndone\nsay "{n}"', "", "3"),
        ("func-with-args",
         'to greet with name and greeting\n    say "{greeting} {name}"\ndone\ngreet with "Bob" and "Hej"', "", "Hej Bob"),
        ("return-value",
         'to double with n\n    give back n times 2\ndone\nsay "{double with 21}"', "", "42"),
        ("thing-default",
         'a Task is a thing with\n    a text\n    finished set to false\ndone\nt is a new Task with text set to "x"\nif the finished of t is false then say "ny"', "", "ny"),
        ("field-lvalue-chain",
         'a Task is a thing with\n    a text\ndone\nts is an empty list\nadd a new Task with text set to "a" to ts\nset the text of item 1 of ts to "z"\nsay "{the text of item 1 of ts}"', "", "z"),
        ("remove-item", 'xs is [1, 2, 3]\nremove item 2 of xs\nsay "{xs}"', "", "[1, 3]"),
        ("check-when-eq",
         'c is "vis"\ncheck the c\n    when it is "vis"\n        say "v"\n    otherwise\n        say "?"\ndone', "", "v"),
        ("check-when-startswith",
         'c is "slet 2"\ncheck the c\n    when it starts with "slet"\n        say "s"\n    otherwise\n        say "?"\ndone', "", "s"),
        ("check-when-isnum-neg",
         'c is "abc"\ncheck the c\n    when it is a number\n        say "tal"\n    when it is not a number\n        say "ikke tal"\n    otherwise\n        say "?"\ndone', "", "ikke tal"),
        ("check-otherwise", 'c is "??"\ncheck the c\n    when it is "x"\n        say "x"\n    otherwise\n        say "nej"\ndone', "", "nej"),
        ("try-catch-div0",
         'try\n    x is 1 divided by 0\nif it fails as err\n    say "fanget"\ndone', "", "fanget"),
        ("string-starts-ends",
         's is "hello world"\nif s starts with "hell" then say "a"\nif s ends with "orld" then say "b"', "", "a\nb"),
        ("contains", 'if "banana" contains "ana" then say "y"', "", "y"),
        ("and-or",
         'if 1 is less than 2 and 2 is less than 3 then say "ja"\nif 1 is greater than 2 or 3 is greater than 2 then say "jo"', "", "ja\njo"),
        ("unless", 'unless 1 is greater than 2 then say "u"', "", "u"),
        ("nothing-check",
         'x is nothing\nif x is nothing then say "tom"', "", "tom"),
        ("wait-parse-only", "wait 0 seconds", "", None),
    ]
    for name, src, stdin, expect in cases:
        path = prog(src, tmp)
        p = nova(["run", path], stdin=stdin, cwd=tmp)
        ok = p.returncode == 0 and (expect is None or expect in p.stdout)
        check(f"phrase/{name}", ok, f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")

    p = nova(["run", prog('x is a random number between 1 and 10\nsay "{x}"', tmp)],
             cwd=tmp)
    seeded = p.stdout.strip()
    q = nova(["run", prog('x is a random number between 1 and 10\nsay "{x}"', tmp), "--seed", "123"],
             cwd=tmp)
    q2 = nova(["run", prog('x is a random number between 1 and 10\nsay "{x}"', tmp), "--seed", "123"],
              cwd=tmp)
    check("phrase/random-deterministic",
          q.stdout == q2.stdout and q.stdout.strip() != "",
          f"{q.stdout!r} vs {q2.stdout!r}")
    del seeded  # (frø-fri kørsel behøver ikke matche noget bestemt)


def test_undo_redo(tmp):
    src = (
        'track x\n'
        'x is 1\n'
        'x is 2\n'
        'undo the last change to x\n'
        'say "{x}"\n'
        'redo the last change to x\n'
        'say "{x}"\n'
    )
    p = nova(["run", prog(src, tmp)], cwd=tmp)
    check("feature/undo-redo", p.returncode == 0 and p.stdout == "1\n2\n",
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")

    src2 = (
        'track xs\n'
        'xs is an empty list\n'
        'add 1 to xs\n'
        'add 2 to xs\n'
        'undo the last change to xs\n'
        'say "{the length of xs}"\n'
    )
    p = nova(["run", prog(src2, tmp)], cwd=tmp)
    check("feature/undo-list", p.returncode == 0 and "1" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")


def test_contracts(tmp):
    ok_src = (
        'to fill with n\n'
        '    requires n is at least 1\n'
        '    ensures n is 10\n'
        '    set n to 10\n'
        'done\n'
        'fill with 5\n'
        'say "ok"\n'
    )
    p = nova(["run", prog(ok_src, tmp)], cwd=tmp)
    check("contract/ensures-deferred-pass",
          p.returncode == 0 and "ok" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")

    fail_src = (
        'to broken with n\n'
        '    requires n is at least 5\n'
        'done\n'
        'broken with 2\n'
    )
    p = nova(["run", prog(fail_src, tmp)], cwd=tmp)
    check("contract/requires-fails",
          p.returncode == 1 and "requires" in p.stderr,
          f"rc={p.returncode} err={p.stderr!r}")

    post_fail = (
        'to broken with n\n'
        '    ensures n is at least 10\n'
        'done\n'
        'broken with 2\n'
    )
    p = nova(["run", prog(post_fail, tmp)], cwd=tmp)
    check("contract/ensures-fails",
          p.returncode == 1 and "ensures" in p.stderr,
          f"rc={p.returncode} err={p.stderr!r}")


def test_io_errors(tmp):
    src = (
        'try\n'
        '    raw is the contents of "findes_ikke.json" parsed as json\n'
        'if it fails as err\n'
        '    say "fanget io-fejl"\n'
        'done\n'
    )
    p = nova(["run", prog(src, tmp)], cwd=tmp)
    check("io/read-missing-catchable",
          p.returncode == 0 and "fanget io-fejl" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")

    store_src = 'xs is [1, 2]\nstore xs in "ud.json" as json\nsay "gemt"'
    p = nova(["run", prog(store_src, tmp)], cwd=tmp)
    json_ok = False
    jp = os.path.join(tmp, "ud.json")
    if os.path.exists(jp):
        with open(jp, encoding="utf-8") as f:
            json_ok = json.load(f) == [1, 2]
    check("io/store-json-roundtrip", p.returncode == 0 and json_ok,
          f"rc={p.returncode} out={p.stdout!r}")

    read_src = ('raw is the contents of "ud.json" parsed as json\n'
                'say "{item 2 of raw}"')
    p = nova(["run", prog(read_src, tmp)], cwd=tmp)
    check("io/read-json-roundtrip", p.returncode == 0 and "2" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")

    bad_json = 'raw is the contents of "ud.json" parsed as json'  # fil er nu gyldig
    corrupt = os.path.join(tmp, "corrupt.json")
    with open(corrupt, "w", encoding="utf-8") as f:
        f.write("{ikke json")
    src_bad = 'raw is the contents of "corrupt.json" parsed as json'
    p = nova(["run", prog(src_bad, tmp)], cwd=tmp)
    check("io/bad-json-clean-error",
          p.returncode == 1 and "json" in p.stderr.lower(),
          f"rc={p.returncode} err={p.stderr!r}")
    del bad_json


# ------------------------------------------------------------- golden dumps

def test_golden(update=False):
    """B05: kanonisk AST-dump sammenlignes byte-for-byte med expected-filer."""
    sources = sorted(f for f in os.listdir(GOLDEN_DIR) if f.endswith(".nova"))
    check("golden/corpus-exists", len(sources) >= 16, f"fandt {len(sources)}")
    for name in sources:
        path = os.path.join(GOLDEN_DIR, name)
        expected_path = path + ".ast.txt"
        p1 = nova(["parse", path])
        if p1.returncode != 0:
            check(f"golden/{name}", False, f"parse rc={p1.returncode} err={p1.stderr!r}")
            continue
        p2 = nova(["parse", path])  # determinisme: to kørsler → samme output
        if p1.stdout != p2.stdout:
            check(f"golden/{name}", False, "dump er ikke-deterministisk")
            continue
        if update or not os.path.exists(expected_path):
            with open(expected_path, "w", encoding="utf-8", newline="\n") as f:
                f.write(p1.stdout)
            check(f"golden/{name}", True, "(opdateret)")
            continue
        with open(expected_path, encoding="utf-8") as f:
            expected = f.read()
        if p1.stdout == expected:
            check(f"golden/{name}", True)
        else:
            exp = expected.splitlines()
            got = p1.stdout.splitlines()
            diff_at = next((i for i in range(max(len(exp), len(got)))
                            if (exp[i] if i < len(exp) else None) != (got[i] if i < len(got) else None)), 0)
            check(f"golden/{name}", False,
                  f"diff ved linje {diff_at + 1}: forventet {exp[diff_at:diff_at+2]!r} "
                  f"fik {got[diff_at:diff_at+2]!r} - kor med --update-goldens hvis aendringen er bevidst")


def test_b05_regressions(tmp):
    """Regressioner opdaget af golden-korpusset."""
    cases = [
        ("bom-tolerated", "\ufeffx is 5\nsay \"{x}\"", "", "5"),
        ("paren-expr", 'c is (1 plus 2) times 3\nsay "{c}"', "", "9"),
        ("multiplied-by", 'b is 2 multiplied by 3\nsay "{b}"', "", "6"),
        ("my-declaration", "my score is 0\nsay \"{score}\"", "", "0"),
        ("at-least-most",
         'if 5 is at least 4 then say "a"\nif 4 is at most 5 then say "b"', "", "a\nb"),
    ]
    for name, src, stdin, expect in cases:
        p = nova(["run", prog(src, tmp)], stdin=stdin, cwd=tmp)
        check(f"regression/{name}",
              p.returncode == 0 and (expect is None or expect in p.stdout),
              f"rc={p.returncode} out={p.stdout!r} err={p.stderr!r}")


# ---------------------------------------------------------- fejlmeddelelser

def test_error_catalog(tmp):
    """B01: alle fejl = sætning + fix-hint; aldrig rå Python-exceptions."""
    # (navn, kilde, stdin, påkrævede fragmenter i stderr)
    cases = [
        ("unknown-var", 'say "{navn}"', "", ["findes ikke", "erklær"]),
        ("unknown-var-suggest", "tries is 0\nsay \"{trie}\"", "", ["mente du 'tries'"]),
        ("unknown-func", 'bake with 1', "", ["findes ikke", "'to"]),
        ("unknown-func-suggest",
         'to greet with name\n    say "{name}"\ndone\ngreat with "x"', "",
         ["mente du 'greet'"]),
        ("arg-count", 'to f with x\n    say "{x}"\ndone\nf()', "", ["forventer 1", "fik 0"]),
        ("div-zero", 'say "{1 divided by 0}"', "", ["division med nul", "nævneren"]),
        ("type-mismatch", 'say "{1 + \\"a\\"}"', "", ["lægge", "to tal eller to tekster"]),
        ("item-bounds", 'xs is [1]\nsay "{item 5 of xs}"', "", ["findes ikke", "gyldige numre"]),
        ("field-missing",
         'a T is a thing with\n    a text\ndone\nx is a new T\nsay "{the txt of x}"', "",
         ["har ikke feltet", "gyldige felter", "text"]),
        ("not-bool-condition", 'if 5 then say "ja"', "", ["true eller false", "sammenligning"]),
        ("contract-requires",
         'to f with n\n    requires n is at least 5\ndone\nf with 2', "",
         ["requires-kontrakt fejlede", "var ikke sand"]),
        ("contract-ensures",
         'to f with n\n    ensures n is at least 10\ndone\nf with 2', "",
         ["ensures-kontrakt fejlede", "garantien"]),
        ("lexer-unterminated", 'say "ups', "", ["uafsluttet streng", "anførselstegn"]),
        ("lexer-newline-in-string", 'say "ups\n"', "", ["nylinje inde i streng"]),
        ("lexer-bad-char", "x @ y", "", ["tegnet '@' er ikke gyldigt", "tastefejl"]),
        ("parse-missing-done", "repeat 2 times\n    say \"hei\"", "", ["mangler 'done'"]),
        ("parse-eol", 'x is 5 plus plus 3', "", ["forventede linjeslut", "én sætning pr. linje"]),
        ("parse-expect-word", "if x is 1 than say \"ja\"", "", ["forventede 'then'", "ordlyden"]),
        ("io-missing-file", 'raw is the contents of "findes_ikke.txt"', "", ["findes ikke"]),
        ("undo-empty", "track x\nx is 1\nundo the last change to x\nundo the last change to x", "",
         ["ingen ændringer at undo"]),
    ]
    for name, src, stdin, fragments in cases:
        p = nova(["run", prog(src, tmp)], stdin=stdin, cwd=tmp)
        ok = (p.returncode == 1
              and all(fr in p.stderr for fr in fragments)
              and "Traceback" not in p.stderr
              and "linje" in p.stderr
              and p.stderr.lstrip().startswith(("Lexer-fejl", "Parser-fejl", "Nova-fejl")))
        check(f"error/{name}", ok, f"rc={p.returncode} err={p.stderr[:220]!r}")

    # exit-koder: parse-fejl ≠ runtime-fejl ≠ CLI-brugsfejl
    bad_syntax = prog("say )(", tmp)
    p = nova(["run", bad_syntax], cwd=tmp)
    check("error/exitcode-syntax", p.returncode == 1 and p.stdout == "", f"rc={p.returncode}")
    p = nova(["run"], cwd=tmp)
    check("error/exitcode-usage", p.returncode == 2, f"rc={p.returncode}")


def test_reserved_words(tmp):
    """B02: reserverede ord kan ikke binde som navne — med klar fejlmeddelelse."""
    bad = [
        ("decl-var", "to is 5"),
        ("set-var", "set done to 5"),
        ("add-target", "add 1 to and"),
        ("loop-var", "repeat for each in in [1]"),
        ("counting-var", "repeat with from from 1 to 2"),
        ("track", "track done"),
        ("func-name", "to if\n    say \"x\"\ndone"),
        ("param-name", 'to greet with name and then\n    say "x"\ndone'),
        ("thing-name", 'a done is a thing with\n    a text\ndone'),
        ("field-name", 'a T is a thing with\n    a done\ndone'),
        ("try-errname", 'try\n    x is 1 divided by 0\nif it fails as done\n    say "e"\ndone'),
        ("new-thing-reserved", "a Task is a thing with\n    a text\ndone\nx is a new the"),
    ]
    for name, src in bad:
        p = nova(["run", prog(src, tmp)], cwd=tmp)
        ok = (p.returncode == 1
              and "reserveret ord" in p.stderr
              and "vælg et andet navn" in p.stderr
              and "Traceback" not in p.stderr)
        check(f"reserved/{name}", ok, f"rc={p.returncode} err={p.stderr[:160]!r}")

    # bevidst tilladte ord (ikke i reservationslisten)
    allowed = [
        ("number", "number is 3", "3"),
        ("length", "length is 7", "7"),
        ("first", "first is 1", "1"),
        ("count", "count is 0", "0"),
        ("answer", 'answer is ask "> "', ""),
        ("mark", 'mark is "[ ]"', "[ ]"),
    ]
    for var, decl, expect in allowed:
        src = f'{decl}\nsay "{{{var}}}"'
        p = nova(["run", prog(src, tmp)],
                 stdin="\n" if expect == "" else "", cwd=tmp)
        check(f"reserved/allowed:{var}",
              p.returncode == 0 and expect in p.stdout,
              f"rc={p.returncode} err={p.stderr[:120]!r}")


def test_shorthand(tmp):
    """C01: kompakt skin — samme AST, symbol-operatorer, =, .felt."""
    cases = [
        ("assign-eq", 'x = 10\nsay "{x}"', "", "10"),
        ("reassign-eq", "x = 1\nx = x + 1\nsay \"{x}\"", "", "2"),
        ("arith-symbols", 'n = 7 + 3 * 2 - 4 / 2\nsay "{n}"', "", "11"),
        ("mod-symbol", 'r = 10 % 3\nsay "{r}"', "", "1"),
        ("compare-symbols",
         'x = 5\nif x > 3 then say "a"\nif x <= 5 then say "b"\n'
         'if x == 5 then say "c"\nif x != 4 then say "d"\nif x < 6 then say "e"\n'
         'if x >= 5 then say "f"', "", "a\nb\nc\nd\ne\nf"),
        ("logic-symbols",
         't = true\nf = false\nif t && !f then say "ja"\nif f || t then say "jo"', "",
         "ja\njo"),
        ("unary-minus", 'x = 5\ny = -x + 12\nsay "{y}"', "", "7"),
        ("unary-bang", 'ok = false\nif !ok then say "nej"', "", "nej"),
        ("negative-literal", 'z = -4 plus 1\nsay "{z}"', "", "-3"),
        ("paren-grouping", 'v = (2 + 3) * 4\nsay "{v}"', "", "20"),
        ("mixed-word-symbol", 'm = 2 times 3 + 1\nk = 2 * 3 plus 1\nif m == k then say "lig"', "", "lig"),
        ("string-plus", 'h = "hej" + " " + "du"\nsay h', "", "hej du"),
        ("dotted-field-read",
         'a Task is a thing with\n    a text\ndone\nt = a new Task with text set to "laes"\nsay t.text', "",
         "laes"),
        ("dotted-field-write",
         'a Task is a thing with\n    a text\ndone\nt = a new Task\nt.text = "skriv"\nsay the text of t', "",
         "skriv"),
        ("dotted-chain",
         'an Inner is a thing with\n    a label\ndone\n'
         'a Box is a thing with\n    an inside set to nothing\ndone\n'
         'b = a new Box\nb.inside = a new Inner\nb.inside.label = "dybt"\nsay b.inside.label', "", "dybt"),
        ("semicolon-stmt-sep", 'p = 1; q = 2; say "{p plus q}"', "", "3"),
    ]
    for name, src, stdin, expect in cases:
        p = nova(["run", prog(src, tmp)], stdin=stdin, cwd=tmp)
        check(f"shorthand/{name}",
              p.returncode == 0 and (expect is None or expect in p.stdout),
              f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # C02-forløber: kryds-skin-par skal give BYTEIDENTISKE AST-dumps
    pairs = [
        ('n1 = 10', 'n1 is 10'),
        ('s = 1 + 2 * 3', 's is 1 plus 2 times 3'),
        ('cmp = x > 3 && y < 4 || ok',
         'cmp is x is greater than 3 and y is less than 4 or ok'),
        ('t.text = "hej"', 'set the text of t to "hej"'),
        ('u = -q + 2', 'u is 0 minus q plus 2'),
        ('u = q? + 1', 'set u to q plus 1?'),
        ('m = tools-module.twice(21)', 'set m to tools-module.twice(21)'),
        ('c = a copy of xs', 'set c to a copy of xs'),
    ]
    for i, (short, natural) in enumerate(pairs, 1):
        ps = nova(["parse", prog(short, tmp)], cwd=tmp)
        pn = nova(["parse", prog(natural, tmp)], cwd=tmp)
        check(f"equivalence/pair{i}",
              ps.returncode == 0 and pn.returncode == 0 and ps.stdout == pn.stdout,
              f"kort={ps.stdout!r} naturlig={pn.stdout!r}")


def test_optional(tmp):
    """C03: Optional/? — hele-udtryksgift; kun fravær-af-værdi propagater
    (specs/error_handling.md §2.1). Uden ? fejler det stadig med fix-hint."""
    cases = [
        # gift-sagen der ville crashet uden grænse: NumVal("abc") → nothing, plus 1
        ("poison-guarded",
         'answer is "abc"\nn = the number value of answer? + 1\n'
         'if n is nothing then say "tom"', "", "tom"),
        ("value-passes", 'v = the number value of "41"? + 1\nsay "{v}"', "", "42"),
        ("natural-spelling",
         'answer is "7"\nn is the number value of answer? plus 1\nsay "{n}"', "", "8"),
        ("field-of-nothing-guarded",
         'a Box is a thing with\n    an inside set to nothing\ndone\n'
         'b = a new Box\nsay "{b.inside.label?}"', "", "nothing"),
        ("interpolation-guarded",
         'maybe is nothing\nsay "{the text of maybe?}"', "", "nothing"),
        ("eq-nothing-still-works",
         'x is nothing\nif x is nothing then say "tom"', "", "tom"),
        ("double-marker", 'k = the number value of "5"?? * 2\nsay "{k}"', "", "10"),
        ("marker-position-free",
         'q = 4\nu = q + 1?\nsay "{u}"', "", "5"),
        ("logic-error-not-covered",
         'xs = [1]\nn = item 9 of xs?\nsay "{n}"', "", None),  # fejler stadig → rc=1
    ]
    for name, src, stdin, expect in cases:
        p = nova(["run", prog(src, tmp)], stdin=stdin, cwd=tmp)
        if name == "logic-error-not-covered":
            ok = (p.returncode == 1 and "item 9" in p.stderr
                  and "Traceback" not in p.stderr)
        else:
            ok = p.returncode == 0 and (expect is None or expect in p.stdout)
        check(f"optional/{name}", ok,
              f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # uden ? : samme udtryk giver venlig sætning + fix-hint (aldrig crash)
    fail_cases = [
        ("unguarded-arith", 'answer is "abc"\nn is the number value of answer plus 1',
         ["kan ikke regne med 'nothing'", "'?'"]),
        ("unguarded-field", 'maybe is nothing\nsay "{the text of maybe}"',
         ["fra nothing", "'?'"]),
    ]
    for name, src, fragments in fail_cases:
        p = nova(["run", prog(src, tmp)], cwd=tmp)
        ok = (p.returncode == 1
              and all(fr in p.stderr for fr in fragments)
              and "Traceback" not in p.stderr
              and "linje" in p.stderr)
        check(f"optional/{name}", ok, f"rc={p.returncode} err={p.stderr[:220]!r}")


def test_modules(tmp):
    """C05: moduler — the X-module in "fil", navnerum, cirkulær import-fejl."""
    def write_mod(fname, src):
        with open(os.path.join(tmp, fname), "w", encoding="utf-8") as f:
            f.write(src)

    write_mod("tools-module.nova",
              'to twice with n\n    give back n times 2\ndone\nanswer is 42\n')
    write_mod("counter-module.nova",
              'x is 1\nto getx\n    give back x\ndone\n')
    write_mod("ping-module.nova", 'say "ping"\n')
    write_mod("a-module.nova", 'the b-module in "b-module.nova"\n')
    write_mod("b-module.nova", 'the a-module in "a-module.nova"\n')

    # 1) import + navnerums-kald + felt-læsning af modul-variabel
    src = ('the tools-module in "tools-module.nova"\n'
           'say "{tools-module.twice(21)}"\n'
           'say "{the answer of the tools-module}"\n')
    p = nova(["run", prog(src, tmp)], cwd=tmp)
    check("module/import-call-read",
          p.returncode == 0 and p.stdout == "42\n42\n",
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # 2) separate navnerum — modulet kan ikke se/forurene hovedprogrammets navne
    src = ('x is 99\n'
           'the counter-module in "counter-module.nova"\n'
           'say "{x}"\n'
           'say "{counter-module.getx()}"\n')
    p = nova(["run", prog(src, tmp)], cwd=tmp)
    check("module/namespace-isolation",
          p.returncode == 0 and p.stdout == "99\n1\n",
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # 3) idempotent: dobbelt import køres én gang
    src = ('the ping-module in "ping-module.nova"\n'
           'the ping-module in "ping-module.nova"\n'
           'say "klar"\n')
    p = nova(["run", prog(src, tmp)], cwd=tmp)
    check("module/idempotent",
          p.returncode == 0 and p.stdout.count("ping") == 1 and "klar" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # 4) kædet import: modul importerer undermodul relativt til sig selv
    os.makedirs(os.path.join(tmp, "sub"), exist_ok=True)
    write_mod("sub/inner-module.nova", 'mark is "indre"\n')
    write_mod("outer-module.nova",
              'the inner-module in "sub/inner-module.nova"\n'
              'to pick\n    give back the mark of the inner-module\ndone\n')
    src = ('the outer-module in "outer-module.nova"\n'
           'say "{outer-module.pick()}"\n')
    p = nova(["run", prog(src, tmp)], cwd=tmp)
    check("module/nested-relative",
          p.returncode == 0 and "indre" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # fejltilfælde: (navn, filer, hovedprogram, påkrævede fragmenter)
    err_cases = [
        ("circular", None,
         'the a-module in "a-module.nova"',
         ["cirkulær import", "a-module.nova"]),
        ("missing-file", None,
         'the ghost-module in "ghost-module.nova"',
         ["ghost-module.nova", "findes ikke"]),
        ("mains-forbidden", [("bad-module.nova", "when the program starts\n    say \"x\"\ndone\n")],
         'the bad-module in "bad-module.nova"',
         ["modul", "when the program starts"]),
        ("unknown-member", None,
         'the tools-module in "tools-module.nova"\nsay "{tools-module.twice(1, 2)}"',
         ["forventer 1", "fik 2"]),
        ("non-module-call", None,
         'n is 5\nsay "{n.foo()}"',
         ["ikke et modul", "the n-module"]),
        ("bad-module-name", None,
         'the tools in "tools-module.nova"',
         ["'-module'", "tools.nova"]),
    ]
    for name, files, src, fragments in err_cases:
        if files:
            for fn, fsrc in files:
                write_mod(fn, fsrc)
        p = nova(["run", prog(src, tmp)], cwd=tmp)
        ok = (p.returncode == 1
              and all(fr in p.stderr for fr in fragments)
              and "Traceback" not in p.stderr
              and "linje" in p.stderr)
        check(f"module/{name}", ok, f"rc={p.returncode} err={p.stderr[:220]!r}")

    # parse-dump: import-sætning og modulkald i golden-korpusset (19-modules)
    p = nova(["parse", os.path.join(GOLDEN_DIR, "19-modules.nova")], cwd=tmp)
    check("module/golden-parses", p.returncode == 0 and "UseModule" in p.stdout
          and "ModuleCall" in p.stdout, f"rc={p.returncode} out={p.stdout[:120]!r}")


def test_stdlib(tmp):
    """B03: use binder ægte stdlib-navnerum (json/file/random/time/math).
    C06-C07-C08 udvider tabellen i specs/standard_library.md §0a."""
    ok_cases = [
        ("json-roundtrip",
         'use the standard json library\n'
         's is json.stringify([1, "a"])\n'
         'v is json.parse(s)\n'
         'say "{item 2 of v} {the length of v}"', "", "a 2"),
        ("random-between",
         'use the standard random library\nx is random.between(5, 5)\nsay "{x}"',
         "", "5"),
        ("random-pick",
         'use the standard random library\nxs is [7]\nif random.pick(xs) is 7 then say "pick ok"',
         "", "pick ok"),
        ("math-sqrt",
         'use the standard math library\nsay "{math.sqrt(9)}"', "", "3"),
        ("time-now",
         'use the standard time library\nif time.now() is greater than 0 then say "tikker"',
         "", "tikker"),
        ("file-write-read-exists",
         'use the standard file library\n'
         'file.write("stdlib-hilsen.txt", "hej fra fil")\n'
         'say file.read("stdlib-hilsen.txt")\n'
         'if file.exists("stdlib-hilsen.txt") then say "findes"', "",
         "hej fra fil\nfindes"),
        ("double-use-idempotent",
         'use the standard json library\nuse standard json\n'
         'say "{json.stringify(42)}"', "", "42"),
        ("catchable-error",
         'use the standard json library\n'
         'try\n    v is json.parse("{ike gyldig")\n'
         'if it fails as err\n    say "fanget: {err}"\ndone', "", "fanget:"),
        # --- C06: text ---
        ("text-case-trim",
         'use the standard text library\n'
         'say text.upper("hej")\nsay text.lower("HEJ")\n'
         'say "[" + text.trim("  x  ") + "]"', "", "HEJ\nhej\n[x]"),
        ("text-split-join",
         'use the standard text library\n'
         'parts is text.split("a,b,c", ",")\n'
         'say "{item 2 of parts}"\nsay text.join(parts, "-")', "", "b\na-b-c"),
        ("text-replace-contains-length",
         'use the standard text library\n'
         'say text.replace("banana", "a", "o")\n'
         'if text.contains("banana", "ana") then say "ja"\n'
         'if text.contains("banana", "xyz") is false then say "nej"\n'
         'b is "banana"\nsay "{text.length(b)}"', "", "bonono\nja\nnej\n6"),
        ("text-at-slice",
         'use the standard text library\n'
         'say text.at("abcdef", 2)\n'
         'say text.slice("abcdef", 2, 4)', "", "b\nbcd"),
        # --- C07: list ---
        ("list-sort",
         'use the standard list library\n'
         'nums is list.sort([3, 1, 2])\nsay "{nums}"\n'
         'ords is list.sort(["b", "a"])\nsay "{ords}"', "", "[1, 2, 3]\n[a, b]"),
        ("list-reverse",
         'use the standard list library\nsay "{list.reverse([1, 2, 3])}"',
         "", "[3, 2, 1]"),
        ("list-min-max",
         'use the standard list library\n'
         'say "{list.min([5, 2, 9])} {list.max([5, 2, 9])}"', "", "2 9"),
        # --- C08: math/time/random fyldes op ---
        ("math-extended",
         'use the standard math library\n'
         'say "{math.abs(0 - 5)} {math.floor(2.7)} {math.ceil(2.1)}"\n'
         'say "{math.pow(2, 10)}"\n'
         'say "{math.PI}"', "", "5 2 3\n1024\n3.14159"),
        ("random-shuffle-copy",
         'use the standard random library\n'
         'xs is [1, 2, 3]\ny is random.shuffle(xs)\n'
         'if the length of xs is 3 then say "orig-ok"\n'
         'say "y har {the length of y} ting"', "", "orig-ok\ny har 3 ting"),
        ("random-shuffle-seeded",
         'use the standard random library\n'
         'y is random.shuffle([1, 2, 3, 4, 5])\nsay "{y}"', "", None),
        ("time-sleep",
         'use the standard time library\ntime.sleep(0)\nsay "vågen"',
         "", "vågen"),
        ("list-keys-values",
         'use the standard list library\nuse the standard file library\n'
         'use the standard json library\n'
         'raw is the contents of "person.json" parsed as json\n'
         'say "{list.keys(raw)}"\nsay "{list.values(raw)}"', "",
         "[alder, navn]\n[42, Bo]"),
    ]
    for name, src, stdin, expect in ok_cases:
        if name == "list-keys-values":
            with open(os.path.join(tmp, "person.json"), "w", encoding="utf-8") as f:
                f.write('{"navn": "Bo", "alder": 42}')
        p = nova(["run", prog(src, tmp)], stdin=stdin, cwd=tmp)
        check(f"stdlib/{name}",
              p.returncode == 0 and (expect is None or expect in p.stdout),
              f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    err_cases = [
        ("unknown-lib", "use the standard turbo library",
         ["ukendt standardbibliotek 'turbo'", "file, json, list, math, random, text, time"]),
        ("bad-use-form", "use magic stuff",
         ["ukendt 'use'-form", "use the standard <navn> library"]),
        ("missing-func", "use the standard json library\nsay \"{json.mangle(1)}\"",
         ["har ikke funktionen 'mangle'"]),
        # --- C06: tekst-fejl ---
        ("text-at-bounds", "use the standard text library\nsay \"{text.at('abc', 9)}\"",
         ["plads 9 findes ikke", "gyldige pladser er 1 til 3"]),
        ("text-slice-bounds", "use the standard text library\nsay \"{text.slice('abc', 2, 9)}\"",
         ["gyldige slutværdier er 1 til 3"]),
        ("text-type-error", "use the standard text library\nsay \"{text.upper(5)}\"",
         ["kræver tekst"]),
        # --- C07: liste-fejl ---
        ("list-sort-mixed", 'use the standard list library\nsay "{list.sort([1, \\"a\\"])}"',
         ["blande typer", "tal eller tekst"]),
        ("list-min-empty", "use the standard list library\nsay \"{list.min([])}\"",
         ["ikke-tom liste"]),
        ("list-keys-nondict", "use the standard list library\nsay \"{list.keys([1])}\"",
         ["kræver en databog", "json.parse"]),
        # --- C08: fejl ---
        ("math-abs-nonnum", "use the standard math library\nsay \"{math.abs('x')}\"",
         ["'math.abs' kræver et tal"]),
        ("time-sleep-negative", "use the standard time library\ntime.sleep(0 - 1)",
         ["kan ikke sove et negativt antal"]),
    ]
    for name, src, fragments in err_cases:
        p = nova(["run", prog(src, tmp)], cwd=tmp)
        ok = (p.returncode == 1
              and all(fr in p.stderr for fr in fragments)
              and "Traceback" not in p.stderr)
        check(f"stdlib/{name}", ok, f"rc={p.returncode} err={p.stderr[:220]!r}")

    # C08: shuffle er deterministisk under --seed
    sh = 'use the standard random library\ny is random.shuffle([1, 2, 3, 4, 5])\nsay "{y}"'
    s1 = nova(["run", prog(sh, tmp), "--seed", "99"], cwd=tmp)
    s2 = nova(["run", prog(sh, tmp), "--seed", "99"], cwd=tmp)
    check("stdlib/shuffle-deterministic",
          s1.returncode == 0 and s1.stdout == s2.stdout and s1.stdout != "",
          f"{s1.stdout!r} vs {s2.stdout!r}")


def test_repl(tmp):
    """C09: nova repl — persistent session, :ast/:undo/:quit, multiline via done."""
    def repl(stdin, seed=None):
        args = ["repl"] + (["--seed", str(seed)] if seed is not None else [])
        return nova(args, stdin=stdin, cwd=tmp)

    # echo af udtryk + persistence
    p = repl("1 plus 2\nx is 5\nx plus 1\n:quit\n")
    check("repl/echo-persist",
          p.returncode == 0 and "→ 3" in p.stdout and "→ 6" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # sætninger kører normalt (say), assignments udskriver intet
    p = repl('say "hej"\nx is 2\n:quit\n')
    check("repl/statements",
          p.returncode == 0 and "hej" in p.stdout and "→" not in p.stdout.split("hej")[1],
          f"rc={p.returncode} out={p.stdout!r}")

    # multiline via done
    p = repl('repeat 2 times\n    say "hei"\ndone\n:quit\n')
    check("repl/multiline-done",
          p.returncode == 0 and p.stdout.count("hei") == 2,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # :ast parser uden at køre
    p = repl(":ast 1 plus 2\n:quit\n")
    check("repl/ast-cmd",
          p.returncode == 0 and "Bin" in p.stdout and "'plus'" in p.stdout
          and "→ 3" not in p.stdout,
          f"rc={p.returncode} out={p.stdout[:200]!r}")

    # :undo gendanner forrige tilstand
    p = repl("track x\nx is 1\nx is 2\n:undo\nx\n:quit\n")
    check("repl/undo",
          p.returncode == 0 and "→ 1" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # :undo på tom stak = venlig besked; :help og ukendt kommando
    p = repl(":undo\n:fisk\n:help\n:quit\n")
    check("repl/meta-errors",
          p.returncode == 0 and "ingenting at undo" in p.stdout
          and ":help" in p.stdout and "ukendt kommando" in p.stdout.lower(),
          f"rc={p.returncode} out={p.stdout!r}")

    # fejl dræber ikke sessionen
    p = repl('say "{navn}"\n7 minus 3\n:quit\n')
    check("repl/error-continues",
          p.returncode == 0 and "findes ikke" in p.stdout and "→ 4" in p.stdout,
          f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # EOF uden :quit afslutter pænt; seed giver determinisme
    p = repl("a random number between 1 and 10\n")
    q = repl("a random number between 1 and 10\n", seed=42)
    r = repl("a random number between 1 and 10\n", seed=42)
    check("repl/eof-and-seed",
          p.returncode == 0 and q.returncode == 0 and q.stdout == r.stdout,
          f"rc={p.returncode}/{q.returncode}/{r.returncode} out={q.stdout!r}")


def test_memory(tmp):
    """C13: værdi- vs reference-semantik + 'a copy of X' (specs/memory_model.md §0)."""
    cases = [
        # pin: liste-delelse er ALIAS — ikke kopi
        ("alias-pin-list",
         'xs is [1, 2]\nys is xs\nadd 3 to ys\nsay "{xs}"', "", "[1, 2, 3]"),
        # pin: thing-felter deles ligeledes
        ("alias-pin-thing",
         'a Box is a thing with\n    an inside set to nothing\ndone\n'
         'b is a new Box\nb2 is b\nset the inside of b2 to "rørt"\n'
         'say "{the inside of b}"', "", "rørt"),
        # pin: tal er værdier
        ("value-pin-number",
         'x is 5\ny is x\ny is 9\nif x is 5 then say "uafhængig"', "", "uafhængig"),
        # a copy of X: dyb og uafhængig
        ("copy-independent",
         'xs is [1, 2]\nks is a copy of xs\nadd 9 to ks\n'
         'say "{xs} {ks}"', "", "[1, 2] [1, 2, 9]"),
        ("copy-nested",
         'xs is [[1], [2]]\nks is a copy of xs\n'
         'indre is item 1 of ks\nadd 99 to indre\n'
         'say "{item 1 of xs} {item 1 of ks}"', "", "[1] [1, 99]"),
        ("copy-thing",
         'a Task is a thing with\n    a text set to "start"\ndone\n'
         't is a new Task\nt2 is a copy of t\nset the text of t2 to "ændret"\n'
         'say "{the text of t} {the text of t2}"', "", "start ændret"),
        ("copy-primitives-nothing",
         'n is a copy of 5\nm is a copy of nothing\n'
         'say "{n}"\nif m is nothing then say "tom-kopi"', "", "5\ntom-kopi"),
        ("copy-with-optional",
         'maybe is nothing\nk is a copy of maybe?\nif k is nothing then say "gift"',
         "", "gift"),
    ]
    for name, src, stdin, expect in cases:
        p = nova(["run", prog(src, tmp)], stdin=stdin, cwd=tmp)
        check(f"memory/{name}",
              p.returncode == 0 and (expect is None or expect in p.stdout),
              f"rc={p.returncode} out={p.stdout!r} err={p.stderr[:200]!r}")

    # moduler kan ikke kopieres
    p = nova(["run", prog(
        'use the standard math library\nm is a copy of math', tmp)], cwd=tmp)
    check("memory/copy-module-error",
          p.returncode == 1 and "ikke en værdi" in p.stderr
          and "Traceback" not in p.stderr,
          f"rc={p.returncode} err={p.stderr[:200]!r}")

    # kryds-skin: begge former giver identisk AST
    ps = nova(["parse", prog('c = a copy of xs', tmp)], cwd=tmp)
    pn = nova(["parse", prog('set c to a copy of xs', tmp)], cwd=tmp)
    check("memory/equivalence",
          ps.returncode == 0 and pn.returncode == 0 and ps.stdout == pn.stdout,
          f"kort={ps.stdout[:80]!r} naturlig={pn.stdout[:80]!r}")


# ------------------------------------------------------------ examples

def test_guessing_game():
    path = os.path.join(ROOT, "examples", "guessing_game.nova")
    stdin = "\n".join(["abc"] + [str(i) for i in range(1, 101)]) + "\n"
    p = nova(["--seed", "7", "run", path], stdin=stdin)
    ok = (p.returncode == 0
          and "Jeg tænker" in p.stdout
          and "Det er ikke et tal" in p.stdout
          and "Rigtigt!" in p.stdout
          and "Du brugte" in p.stdout)
    check("example/guessing_game", ok, f"rc={p.returncode} out={p.stdout[:200]!r}")


def test_todo_app():
    workdir = tempfile.mkdtemp()
    path = os.path.join(ROOT, "examples", "todo.nova")
    jp = os.path.join(workdir, "todo.json")

    stdin1 = "tilføj køb brød\nvis\nfærdig 1\nvis\nfarvel\n"
    p1 = nova(["run", path], stdin=stdin1, cwd=workdir)
    lines = p1.stdout
    ok1 = (p1.returncode == 0
           and "[ ] 1) køb brød" in lines.split("[X]")[0]
           and "[X] 1) køb brød" in lines
           and os.path.exists(jp))
    check("example/todo-session", ok1, f"rc={p1.returncode} out={lines[:300]!r}")

    data = {}
    if os.path.exists(jp):
        with open(jp, encoding="utf-8") as f:
            data = json.load(f)
    check("example/todo-persisted-json",
          len(data) == 1 and data[0]["text"] == "køb brød" and data[0]["finished"] is True,
          f"data={data!r}")

    p2 = nova(["run", path], stdin="vis\nfarvel\n", cwd=workdir)
    check("example/todo-reload", p2.returncode == 0 and "[X] 1) køb brød" in p2.stdout,
          f"rc={p2.returncode} out={p2.stdout[:200]!r}")

    p3 = nova(["parse", path])
    check("example/todo-parse-dump", p3.returncode == 0 and "WhenProgramStarts" in p3.stdout,
          f"rc={p3.returncode}")


def main():
    try:
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")
        sys.stderr.reconfigure(encoding="utf-8", errors="replace")
    except Exception:
        pass
    print(f"Nova bootstrap test-suite - ROOT={ROOT}\n")
    update_goldens = "--update-goldens" in sys.argv
    tmp = tempfile.mkdtemp(prefix="nova-tests-")
    try:
        test_cli(tmp)
        test_phrases(tmp)
        test_undo_redo(tmp)
        test_contracts(tmp)
        test_io_errors(tmp)
        test_golden(update=update_goldens)
        test_b05_regressions(tmp)
        test_error_catalog(tmp)
        test_reserved_words(tmp)
        test_shorthand(tmp)
        test_optional(tmp)
        test_modules(tmp)
        test_stdlib(tmp)
        test_repl(tmp)
        test_memory(tmp)
        test_guessing_game()
        test_todo_app()
    finally:
        print()
    total = _passed + _failed
    print(f"{_passed}/{total} bestået" + ("" if _failed == 0 else f" — {_failed} FEJLEDE"))
    return 1 if _failed else 0


if __name__ == "__main__":
    sys.exit(main())

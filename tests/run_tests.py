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
    check("cli/version", p.returncode == 0 and "Nova" in p.stdout and "0.11" in p.stdout,
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
    ]
    for i, (short, natural) in enumerate(pairs, 1):
        ps = nova(["parse", prog(short, tmp)], cwd=tmp)
        pn = nova(["parse", prog(natural, tmp)], cwd=tmp)
        check(f"equivalence/pair{i}",
              ps.returncode == 0 and pn.returncode == 0 and ps.stdout == pn.stdout,
              f"kort={ps.stdout!r} naturlig={pn.stdout!r}")


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
        test_guessing_game()
        test_todo_app()
    finally:
        print()
    total = _passed + _failed
    print(f"{_passed}/{total} bestået" + ("" if _failed == 0 else f" — {_failed} FEJLEDE"))
    return 1 if _failed else 0


if __name__ == "__main__":
    sys.exit(main())

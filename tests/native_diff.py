"""N05: differential harness - native VM vs Python oracle.

Runs a corpus of programs through BOTH engines and compares stdout + exit code.
Each entry may bring extra files (for module-import tests); every run executes
in an isolated temp dir so file-I/O tests stay hermetic. Random is excluded:
the two engines intentionally use different PRNGs (documented divergence).

Usage:
    cargo build (in native/)   # then:
    python tests/native_diff.py
Exit codes: 0 = all identical, 1 = any difference.
"""

import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ORACLE = [sys.executable, os.path.join(ROOT, "bootstrap", "nova_cli.py"), "run"]
NATIVE = [os.path.join(ROOT, "native", "target", "debug", "nova.exe"),
          os.path.join(ROOT, "native", "target", "debug", "nova")]

TOOLS_MODULE = ('to twice with n\n'
                'give back n times 2\n'
                'done\n'
                'answer is 42\n')

# (name, main-source, {extra-filename: source})
CORPUS = [
    ("arith-precedence", 'say 1 plus 2 times 3\nsay (1 plus 2) times 3\nsay 7 divided by 2', {}),
    ("bigint", 'say 99999999999999999999 plus 1', {}),
    ("python-modulo", 'say 0 - 7 mod (0 - 3)', {}),
    ("eq-pinning", 'say true is 1\nsay 1 is 1.0\nsay nothing is nothing', {}),
    ("lists", 'xs = [1, 2]\nyy = xs\nadd 5 to yy\nsay xs\nsay [1] is [1]\nsay [] contains 1', {}),
    ("strings", 'say "ab" plus "cd"\nsay "abc" contains "bc"\nsay "nova" starts with "no"', {}),
    ("if-chain", 'x = 7\nif x is less than 5 then\nsay "low"\notherwise\nsay "mid"\ndone', {}),
    ("loops", 'n = 0\nrepeat 4 times\nset n to n plus 1\ndone\nsay n\ni = 0\nrepeat until i is 3\nset i to i plus 1\ndone\nsay i', {}),
    ("counting", 't = ""\nrepeat with i from 1 to 3\nif i is 1 then set t to t plus "a"\nif i is 2 then set t to t plus "b"\nif i is 3 then set t to t plus "c"\ndone\nsay t', {}),
    ("functions-recursion", 'to fib with nnn\nif nnn is less than 2 then give back nnn\ngive back fib(nnn minus 1) plus fib(nnn minus 2)\ndone\nsay fib(12)', {}),
    ("scope", 'g = 5\nto fff with nnn\nset g to g plus nnn\ndone\nfff(3)\nsay g', {}),
    ("check", 'check "hello"\nwhen it contains "ell"\nsay "yes"\nwhen it is empty\nsay "empty"\notherwise\nsay "no"\ndone', {}),
    ("try-catch", 'try\nsay 1 divided by 0\nif it fails as eee\nsay eee\ndone', {}),
    ("contracts", 'to pos with nnn\nrequires nnn is greater than 0\ngive back nnn times 2\ndone\nsay pos(21)', {}),
    ("track-history", 'xx is 1\ntrack xx\nset xx to 2\nset xx to 3\nundo the last change to xx\nsay xx\nredo the last change to xx\nsay xx', {}),
    ("things", 'a dog is a thing with\nname set to "rex"\ndone\ndd is a new dog\nsay dd.name\nset the name of dd to "max"\nsay dd.name', {}),
    ("display-rules", 'say 1 is 1\nsay nothing\nsay [1, [2, "a"]]\nwrite "a"\nwrite "b"\nsay "c"', {}),
    ("syntax-error-exit", 'say 1 plus', {}),
    ("inline-otherwise",
     'x = 3\n'
     'if x is less than 5 then say "low" otherwise say "high"\n'
     'if x is greater than 5 then say "big" otherwise if x is 3 then say "three"', {}),
    # ---- N04f: stdlib v0 + modules ----
    ("stdlib-json",
     'use the standard json library\n'
     's = json.stringify([1, "a"])\nv = json.parse(s)\n'
     'say "{item 2 of v} {the length of v}"\nsay json.stringify("tekst")', {}),
    ("stdlib-text",
     'use the standard text library\n'
     'say text.upper("hej")\nsay text.lower("HEJ")\nsay "[" + text.trim("  x ") + "]"\n'
     'p = text.split("a,b,c", ",")\nsay "{item 2 of p}"\nsay text.join(p, "-")\n'
     'say "{text.length("banana")}"\nif text.contains("banana", "ana") then say "y"\n'
     'say text.at("abcdef", 2)\nsay text.slice("abcdef", 2, 4)', {}),
    ("stdlib-list",
     'use the standard list library\nuse the standard json library\n'
     'say list.sort([3, 1, 2])\nsay list.sort(["b", "a"])\n'
     'say list.reverse([1, 2, 3])\nsay "{list.min([5, 2, 9])} {list.max([5, 2, 9])}"', {}),
    ("stdlib-file",
     'use the standard file library\n'
     'file.write("diff-hilsen.txt", "hej fra fil")\n'
     'say file.read("diff-hilsen.txt")\n'
     'if file.exists("diff-hilsen.txt") then say "findes"\n'
     'if file.exists("mangler-helt.txt") then say "ups" otherwise say "ok-mangler"', {}),
    ("stdlib-math",
     'use the standard math library\n'
     'say "{math.sqrt(16)} {math.floor(2.7)} {math.ceil(2.1)}"\n'
     'say "{math.abs(0 - 5)}"\nsay math.pow(2, 8)', {}),
    ("module-basic",
     'the tools-module in "tools-module.nova"\n'
     'say tools-module.twice(21)\nsay "{the answer of the tools-module}"',
     {"tools-module.nova": TOOLS_MODULE}),
    ("module-namespace-isolation",
     'x = 99\nthe counter-module in "counter-module.nova"\n'
     'say "{x} {counter-module.getx()}"',
     {"counter-module.nova": 'x = 1\nto getx\ngive back x\ndone\n'}),
    ("module-circular-error",
     'the a-module in "a-module.nova"',
     {"a-module.nova": 'the b-module in "b-module.nova"\n',
      "b-module.nova": 'the a-module in "a-module.nova"\n'}),
    ("module-missing-error", 'the ghost-module in "ghost-module.nova"', {}),
    ("optional-guarded",
     'answer = "abc"\nk = the number value of answer? + 1\nif k is nothing then say "tom"', {}),
    ("copy-deep",
     'xs = [[1]]\nks = a copy of xs\ninner = item 1 of ks\nadd 7 to inner\nsay "{xs} {ks}"', {}),
]


def engine_output(cmd, workdir):
    path = os.path.join(workdir, "main.nova")
    env = dict(os.environ, PYTHONIOENCODING="utf-8", PYTHONUTF8="1")
    p = subprocess.run(cmd + [path], capture_output=True, env=env, cwd=workdir)
    out = p.stdout.replace(b"\r\n", b"\n")
    err = p.stderr.replace(b"\r\n", b"\n")
    return p.returncode, out, err


def find_native():
    for c in NATIVE:
        if os.path.exists(c):
            return c
    print("native binary not found — build with: cargo build (in native/)")
    sys.exit(1)


def _strip_prefixes(text):
    """Remove engine-specific error wrappers, keeping the core sentence.
    Oracle: 'Parser error — line 3, column 11: <sentence>'
    Native: 'nova: line 3: <sentence>' (column alignment tracked separately)
    """
    for pre in ("parser error", "lexer error", "nova error", "nova:"):
        if text.lower().lstrip().startswith(pre):
            # drop any leading blank line(s) plus the engine prefix itself
            text = text.lstrip()
            text = text[len(pre):].lstrip(" \u2014-:")
            break
    low = text.lower()
    if low.startswith("line "):
        rest = text[5:].lstrip("0123456789").lstrip(" :,-\u2014")
        if rest.lower().startswith("column "):
            rest = rest[7:].lstrip("0123456789").lstrip(" :,-\u2014")
        text = rest
    i = 0
    while i < len(text) and not text[i].isalpha():
        i += 1
    j = len(text)
    while j > i and not (text[j - 1].isalnum() or text[j - 1] in "'\"]."):
        j -= 1
    return text[i:j].lower()


def _core(b):
    return _strip_prefixes(b.decode("utf-8", "replace")).encode()[:80]


def main():
    exe = find_native()
    failures = 0
    for name, src, extra in CORPUS:
        workdir = tempfile.mkdtemp(prefix="nova-diff-")
        try:
            with open(os.path.join(workdir, "main.nova"), "w", encoding="utf-8", newline="\n") as f:
                f.write(src)
            for fname, fsrc in extra.items():
                fpath = os.path.join(workdir, fname)
                os.makedirs(os.path.dirname(fpath), exist_ok=True)
                with open(fpath, "w", encoding="utf-8", newline="\n") as f:
                    f.write(fsrc)
            orc_rc, orc_out, orc_err = engine_output(ORACLE, workdir)
            nat_rc, nat_out, nat_err = engine_output([exe, "run"], workdir)
            same_out = orc_out == nat_out
            same_rc_sign = (orc_rc == 0) == (nat_rc == 0)
            err_ok = (orc_err == b"" and nat_err == b"") or (
                orc_err != b"" and nat_err != b"" and _core(nat_err) in _core(orc_err)
            )
            if same_out and same_rc_sign and err_ok:
                print(f"OK    {name}")
            else:
                failures += 1
                print(f"DIFF  {name}")
                if not same_out:
                    print(f"      oracle-out: {orc_out!r}")
                    print(f"      native-out: {nat_out!r}")
                if not same_rc_sign:
                    print(f"      rc: oracle={orc_rc} native={nat_rc}")
                if not err_ok:
                    print(f"      oracle-err: {orc_err!r}")
                    print(f"      native-err: {nat_err!r}")
        finally:
            shutil.rmtree(workdir, ignore_errors=True)
    total = len(CORPUS)
    print(f"{total - failures}/{total} identical")
    sys.exit(0 if failures == 0 else 1)


if __name__ == "__main__":
    main()

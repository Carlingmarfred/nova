"""Nova bootstrap CLI.

Brug:
  python nova_cli.py run <fil.nova> [--seed N]
  python nova_cli.py parse <fil.nova>     (AST-dump)
  python nova_cli.py version
"""

import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from nova_lexer import lex, NovaLexError          # noqa: E402
from nova_parser import parse_source, NovaParseError  # noqa: E402
from nova_interpreter import Interp, NovaError, ExitSignal, NothingSignal  # noqa: E402
from nova_dump import dump_program  # noqa: E402

VERSION = "0.11.0-bootstrap"


def _utf8():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass


def _print_err(prefix, e):
    loc = f"linje {e.line}"
    col = getattr(e, "col", None)
    if col:
        loc += f", kolonne {col}"
    print(f"{prefix} — {loc}: {e.msg}", file=sys.stderr)


def main(argv):
    _utf8()
    seed = None
    args = list(argv)
    while "--seed" in args:
        i = args.index("--seed")
        if i + 1 >= len(args):
            print("--seed kræver et heltal", file=sys.stderr)
            return 2
        try:
            seed = int(args[i + 1])
        except ValueError:
            print(f"ugyldigt --seed '{args[i + 1]}' — brug et heltal", file=sys.stderr)
            return 2
        del args[i:i + 2]

    if args and args[0] == "version":
        print(f"Nova {VERSION} (bootstrap-fortolker i Python)")
        return 0

    if len(args) < 2:
        print(__doc__)
        return 2

    cmd, path = args[0], args[1]

    if cmd == "version":
        print(f"Nova {VERSION} (bootstrap-fortolker i Python)")
        return 0

    try:
        with open(path, "r", encoding="utf-8") as f:
            src = f.read()
    except OSError as e:
        print(f"Kan ikke læse {path}: {e}", file=sys.stderr)
        return 2

    try:
        stmts = parse_source(src)
    except NovaLexError as e:
        _print_err("Lexer-fejl", e)
        return 1
    except NovaParseError as e:
        _print_err("Parser-fejl", e)
        return 1

    if cmd == "parse":
        print(dump_program(stmts))
        return 0

    if cmd == "run":
        interp = Interp(seed=seed, root_dir=os.path.dirname(os.path.abspath(path)))
        try:
            interp.run(stmts)
            return 0
        except NovaError as e:
            print(f"\nNova-fejl — linje {e.line}: {e.msg}", file=sys.stderr)
            return 1
        except NothingSignal as ns:
            print(f"\nNova-fejl — linje {ns.line}: {ns.msg}", file=sys.stderr)
            return 1
        except (NovaLexError, NovaParseError) as e:  # fx fejl i et modul under kørsel
            _print_err("Fejl", e)
            return 1
        except ExitSignal:
            return 0
        except RecursionError:
            print("\nNova-fejl: uendelig rekursion?", file=sys.stderr)
            return 1

    print(f"Ukendt kommando '{cmd}' (brug run/parse/version)", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))

"""Nova bootstrap CLI.

Brug:
  python nova_cli.py run <file.nova> [--seed N]
  python nova_cli.py repl [--seed N]      (interactive session; :quit exits)
  python nova_cli.py parse <file.nova>    (AST dump)
  python nova_cli.py version
"""

import copy
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from nova_lexer import lex, NovaLexError          # noqa: E402
from nova_parser import (parse_source, NovaParseError, Parser,  # noqa: E402
                         ExprStmt)
from nova_interpreter import (Interp, NovaError, ExitSignal, NothingSignal,  # noqa: E402
                              nova_str)
from nova_dump import dump_program  # noqa: E402

VERSION = "0.15.0-bootstrap"


def _utf8():
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except Exception:
        pass


def _print_err(prefix, e):
    loc = f"line {e.line}"
    col = getattr(e, "col", None)
    if col:
        loc += f", column {col}"
    print(f"{prefix} — {loc}: {e.msg}", file=sys.stderr)


# ----------------------------- REPL (C09) -----------------------------

def _is_open_block_error(e):
    "\"\"\"The 'done' family: the block is still open — the REPL keeps reading.\"\"\""
    return isinstance(e, NovaParseError) and "done" in e.msg


def _try_parse_expr_only(src):
    """Parse src som KUN ét udtryk (fx '1 plus 2') — None hvis det ikke er ét."""
    try:
        p = Parser(lex(src))
        expr = p.parse_expr()
        p.skip_newlines()
        if p.peek().kind == "EOF":
            return expr
    except (NovaLexError, NovaParseError):
        pass
    return None


def _repl_eval_echo(interp, expr, undo_stack):
    undo_stack.append((copy.deepcopy(interp.globals.vars),
                       dict(interp.funcs), dict(interp.things)))
    if len(undo_stack) > 100:
        undo_stack.pop(0)
    try:
        val = interp.eval(expr, interp.globals)
        print("→ " + nova_str(val))
    except NovaError as e:
        print(f"Nova error — line {e.line}: {e.msg}")
    except NothingSignal as ns:
        print(f"Nova error — line {ns.line}: {ns.msg}")
    except ExitSignal:
        print("(the program was stopped)")
    except RecursionError:
        print("Nova error: infinite recursion?")


def _repl_meta(text, interp, undo_stack):
    """Handle one ':command'. Return 'quit' if the session should end."""
    c = text[1:].strip().lower()
    name, _, rest = c.partition(" ")
    if name in ("quit", "q"):
        return "quit"
    if name == "help":
        print(":ast <line>  — show the AST dump without running the code")
        print(":undo        — revert the last executed block "
              "(already printed output and file I/O cannot be rolled back)")
        print(":quit (:q)   — exit the REPL")
        print(":help        — this list")
    elif name == "ast":
        if not rest.strip():
            print("usage: :ast <expression or sentence>")
        else:
            try:
                toks = lex(rest)
                try:
                    nodes = [ExprStmt(Parser(toks).parse_expr())]
                except NovaParseError:
                    nodes = parse_source(rest)
                print(dump_program(nodes))
            except (NovaLexError, NovaParseError) as e:
                _print_err("Error", e)
    elif name == "undo":
        if not undo_stack:
            print("nothing to undo — the stack is empty")
        else:
            gvars, funcs, things = undo_stack.pop()
            interp.globals.vars = gvars
            interp.funcs.clear()
            interp.funcs.update(funcs)
            interp.things.clear()
            interp.things.update(things)
            print("(reverted)")
    else:
        print(f"unknown command '{text.strip()}' — try :help")
    return None


def _run_repl(seed):
    """C09: persistent session on one Interp (spec in docs/ARCHITECTURE.md section 10)."""
    interp = Interp(seed=seed)
    undo_stack = []
    buffer = []
    print(f"Nova {VERSION} REPL — write Nova Natural; blocks end with 'done'.")
    print("Meta-commands: :ast <line> · :undo · :quit · :help")
    while True:
        try:
            line = input("..> " if buffer else ">>> ")
        except EOFError:
            print()
            return 0
        except KeyboardInterrupt:
            print()
            buffer = []  # cancel an open block instead of dying
            continue
        if line.strip().startswith(":") and not buffer:
            if _repl_meta(line, interp, undo_stack) == "quit":
                return 0
            continue
        buffer.append(line)
        src = "\n".join(buffer)
        try:
            stmts = parse_source(src)
        except NovaLexError as e:
            buffer = []
            _print_err("Error", e)
            continue
        except NovaParseError as e:
            if _is_open_block_error(e):
                continue  # block still open — keep reading
            expr = _try_parse_expr_only(src)  # fx '1 plus 2' → echo i stedet for fejl
            buffer = []
            if expr is not None:
                _repl_eval_echo(interp, expr, undo_stack)
                continue
            _print_err("Error", e)
            continue
        buffer = []
        if not stmts:
            continue
        echo = (len(stmts) == 1 and isinstance(stmts[0], ExprStmt))
        try:
            if echo:
                _repl_eval_echo(interp, stmts[0].expr, undo_stack)
            else:
                undo_stack.append((copy.deepcopy(interp.globals.vars),
                                   dict(interp.funcs), dict(interp.things)))
                if len(undo_stack) > 100:
                    undo_stack.pop(0)
                interp.run(stmts)
        except NovaError as e:
            print(f"Nova error — line {e.line}: {e.msg}")
        except NothingSignal as ns:
            print(f"Nova error — line {ns.line}: {ns.msg}")
        except ExitSignal:
            print("(the program was stopped)")
        except RecursionError:
            print("Nova error: infinite recursion?")


# ------------------------------ main ------------------------------

def main(argv):
    _utf8()
    seed = None
    args = list(argv)
    while "--seed" in args:
        i = args.index("--seed")
        if i + 1 >= len(args):
            print("--seed needs an integer", file=sys.stderr)
            return 2
        try:
            seed = int(args[i + 1])
        except ValueError:
            print(f"invalid --seed '{args[i + 1]}' — use an integer", file=sys.stderr)
            return 2
        del args[i:i + 2]

    if args and args[0] == "version":
        print(f"Nova {VERSION} (bootstrap-fortolker i Python)")
        return 0

    if args and args[0] == "repl":
        return _run_repl(seed)

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
        print(f"Cannot read {path}: {e}", file=sys.stderr)
        return 2

    try:
        stmts = parse_source(src)
    except NovaLexError as e:
        _print_err("Lexer error", e)
        return 1
    except NovaParseError as e:
        _print_err("Parser error", e)
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
            print(f"\nNova error — line {e.line}: {e.msg}", file=sys.stderr)
            return 1
        except NothingSignal as ns:
            print(f"\nNova error — line {ns.line}: {ns.msg}", file=sys.stderr)
            return 1
        except (NovaLexError, NovaParseError) as e:  # e.g. errors inside a module at run time
            _print_err("Error", e)
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

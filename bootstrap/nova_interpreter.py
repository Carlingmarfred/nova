"""Nova bootstrap fortolker — kører Natural-syntax AST."""

import copy
import difflib
import json as _json
import math as _math
import os
import random as _random
import sys
import time

from nova_parser import (FuncDef, ThingDef, WhenProgramStarts, ItemAt,
                         UseModule, ModuleCall, parse_source,
                         NovaLexError, NovaParseError)


def _suggest(name, candidates):
    """Did-you-mean: nærmeste kandidat eller tom streng."""
    match = difflib.get_close_matches(str(name), [str(c) for c in candidates], n=1, cutoff=0.6)
    return f" — mente du '{match[0]}'?" if match else ""


class NovaError(Exception):
    def __init__(self, line, msg):
        super().__init__(f"linje {line}: {msg}")
        self.line = line
        self.msg = msg


class BreakSignal(Exception):
    pass


class ContinueSignal(Exception):
    pass


class ReturnSignal(Exception):
    def __init__(self, value):
        self.value = value


class ExitSignal(Exception):
    pass


class NothingSignal(Exception):
    """C03: fravær-af-værdi under et '?'-udtryk. Opsluges KUN af QuestionE-
    grænsen; når toppen = venlig sætning + fix-hint (specs/error_handling.md §2.1)."""

    def __init__(self, line, msg):
        super().__init__(msg)
        self.line = line
        self.msg = msg


NOTHING = None


class ThingInstance:
    __slots__ = ("cls", "fields")

    def __init__(self, cls, fields):
        self.cls = cls
        self.fields = fields


class Function:
    __slots__ = ("name", "params", "body")

    def __init__(self, name, params, body):
        self.name = name
        self.params = params
        self.body = body


class ModuleInstance:
    """C05: ét importeret modul — separate navnerum (funcs/things/scope)."""
    __slots__ = ("name", "path", "funcs", "things", "scope")

    def __init__(self, name, path):
        self.name = name
        self.path = path
        self.funcs = {}
        self.things = {}
        self.scope = Scope()  # parent=None: fuldstændig isolation fra hovedprogrammet


class BuiltinFunction:
    """B03: stdlib-funktion implementeret i Python — kaldes via ModuleCall."""
    __slots__ = ("name", "params", "fn")

    def __init__(self, name, params, fn):
        self.name = name
        self.params = params  # parameternavne (til arity-fejl)
        self.fn = fn          # fn(args, line) -> værdi


class Scope:
    __slots__ = ("vars", "parent")

    def __init__(self, parent=None):
        self.vars = {}
        self.parent = parent

    def get(self, name, line):
        s = self
        while s:
            if name in s.vars:
                return s.vars[name]
            s = s.parent
        known = []
        s = self
        while s:
            known.extend(s.vars.keys())
            s = s.parent
        raise NovaError(line, f"variablen '{name}' findes ikke{_suggest(name, known)}"
                              " — tjek stavemåden eller erklær den med 'x is ...'")

    def has(self, name):
        s = self
        while s:
            if name in s.vars:
                return True
            s = s.parent
        return False

    def set(self, name, value):
        s = self
        while s:
            if name in s.vars:
                s.vars[name] = value
                return
            s = s.parent
        self.vars[name] = value  # deklarér hvor den bruges

    def declare(self, name, value):
        self.vars[name] = value


def _is_num(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def nova_str(v):
    if v is NOTHING:
        return "nothing"
    if v is True:
        return "true"
    if v is False:
        return "false"
    if isinstance(v, float):
        return str(int(v)) if v.is_integer() else str(v)
    if isinstance(v, list):
        return "[" + ", ".join(nova_str(x) for x in v) + "]"
    if isinstance(v, ThingInstance):
        return v.cls + "(...)"
    return str(v)


def to_plain(v):
    if isinstance(v, ThingInstance):
        return {k: to_plain(x) for k, x in v.fields.items()}
    if isinstance(v, list):
        return [to_plain(x) for x in v]
    if isinstance(v, dict):
        return {k: to_plain(x) for k, x in v.items()}
    return v


# ---------------- standardbiblioteker (B03; udvides af C06/C07/C08) ----------------
# Hvert bibliotek er en almindelig ModuleInstance — samme kaldsmaskineri som
# C05-moduler (specs/standard_library.md §0a).

def _read_text_file(path, line):
    try:
        with open(path, "r", encoding="utf-8") as f:
            return f.read()
    except FileNotFoundError:
        raise NovaError(line, f"'{path}' findes ikke")
    except IsADirectoryError:
        raise NovaError(line, f"'{path}' er en mappe, ikke en fil")
    except UnicodeDecodeError:
        raise NovaError(line, f"'{path}' er ikke en UTF-8-tekstfil")
    except OSError as err:
        raise NovaError(line, f"kan ikke læse '{path}': {err}")


def _bi_json_parse(args, line):
    v = args[0]
    if not isinstance(v, str):
        raise NovaError(line, "'json.parse' kræver tekst — giv den en streng med json i")
    try:
        return _json.loads(v)
    except _json.JSONDecodeError as err:
        raise NovaError(line, f"ugyldig json (linje {err.lineno}) — tjek teksten, "
                              "eller fang fejlen med 'try ... if it fails'")


def _bi_json_stringify(args, line):
    return _json.dumps(to_plain(args[0]), ensure_ascii=False)


def _bi_file_read(args, line):
    return _read_text_file(nova_str(args[0]), line)


def _bi_file_exists(args, line):
    return os.path.exists(nova_str(args[0]))


def _bi_file_write(args, line):
    path = nova_str(args[0])
    text = args[1]
    if not isinstance(text, str):
        raise NovaError(line, "'file.write' kræver tekst som indhold")
    try:
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)
    except OSError as err:
        raise NovaError(line, f"kan ikke skrive til '{path}': {err}")
    return NOTHING


def _bi_random_between(args, line):
    a, b = args
    if not (_is_num(a) and _is_num(b)):
        raise NovaError(line, "'random.between' kræver to tal")
    return _random.randint(int(a), int(b))


def _bi_random_pick(args, line):
    xs = args[0]
    if not isinstance(xs, list) or not xs:
        raise NovaError(line, "'random.pick' kræver en ikke-tom liste")
    return _random.choice(xs)


def _bi_time_now(args, line):
    return time.time()


def _bi_math_sqrt(args, line):
    n = args[0]
    if not _is_num(n) or n < 0:
        raise NovaError(line, "'math.sqrt' kræver et tal der er 0 eller større")
    return _math.sqrt(n)


def _bi_math_round(args, line):
    n = args[0]
    if not _is_num(n):
        raise NovaError(line, "'math.round' kræver et tal")
    return int(round(n))


# --- C06: text ---

def _bi_text_str(name, v, line):
    if not isinstance(v, str):
        raise NovaError(line, f"'text.{name}' kræver tekst — fandt {nova_str(v)}")
    return v


def _bi_text_upper(args, line):
    return _bi_text_str("upper", args[0], line).upper()


def _bi_text_lower(args, line):
    return _bi_text_str("lower", args[0], line).lower()


def _bi_text_trim(args, line):
    return _bi_text_str("trim", args[0], line).strip()


def _bi_text_split(args, line):
    s = _bi_text_str("split", args[0], line)
    sep = _bi_text_str("split", args[1], line)
    if sep == "":
        raise NovaError(line, "'text.split' kræver en ikke-tom adskiller")
    return s.split(sep)


def _bi_text_join(args, line):
    xs = args[0]
    sep = _bi_text_str("join", args[1], line)
    if not isinstance(xs, list):
        raise NovaError(line, "'text.join' kræver en liste — giv den fx text.split(...) først")
    return sep.join(nova_str(x) for x in xs)


def _bi_text_replace(args, line):
    s = _bi_text_str("replace", args[0], line)
    fra = _bi_text_str("replace", args[1], line)
    til = _bi_text_str("replace", args[2], line)
    if fra == "":
        raise NovaError(line, "'text.replace' kræver en ikke-tom søgetekst")
    return s.replace(fra, til)  # erstatter ALLE forekomster


def _bi_text_length(args, line):
    return len(_bi_text_str("length", args[0], line))


def _bi_text_contains(args, line):
    s = _bi_text_str("contains", args[0], line)
    sub = _bi_text_str("contains", args[1], line)
    return sub in s


def _bi_text_at(args, line):
    s = _bi_text_str("at", args[0], line)
    n = args[1]
    if not _is_num(n):
        raise NovaError(line, "'text.at' kræver et tal som plads")
    i = int(n)
    if i < 1 or i > len(s):
        raise NovaError(line, f"plads {i} findes ikke (teksten har kun {len(s)} tegn)"
                              f" — gyldige pladser er 1 til {max(len(s), 1)}")
    return s[i - 1]


def _bi_text_slice(args, line):
    s = _bi_text_str("slice", args[0], line)
    a, b = args[1], args[2]
    if not (_is_num(a) and _is_num(b)):
        raise NovaError(line, "'text.slice' kræver tal som fra/til (1-baseret, inklusiv)")
    a, b = int(a), int(b)
    if a < 1 or b > len(s) or a > b:
        raise NovaError(line, f"slice {a} til {b} rækker uden for teksten"
                              f" — gyldige slutværdier er 1 til {max(len(s), 1)}")
    return s[a - 1:b]


# --- C07: list ---

def _bi_list_xs(name, v, line):
    if not isinstance(v, list):
        raise NovaError(line, f"'list.{name}' kræver en liste — fandt {nova_str(v)}")
    return v


def _bi_list_sort(args, line):
    xs = _bi_list_xs("sort", args[0], line)
    if all(_is_num(x) for x in xs):
        return sorted(xs)
    if all(isinstance(x, str) for x in xs):
        return sorted(xs)
    typer = ", ".join(sorted({"tal" if _is_num(x) else "tekst" if isinstance(x, str)
                              else nova_str(x) for x in xs}))
    raise NovaError(line, f"'list.sort' kan ikke blande typer ({typer}) — "
                          "giv den en liste med ENTEN tal eller tekst")


def _bi_list_reverse(args, line):
    return list(reversed(_bi_list_xs("reverse", args[0], line)))


def _bi_list_min(args, line):
    xs = _bi_list_xs("min", args[0], line)
    if not xs:
        raise NovaError(line, "'list.min' kræver en ikke-tom liste")
    if not all(_is_num(x) for x in xs):
        raise NovaError(line, "'list.min' kræver en liste af tal")
    return min(xs)


def _bi_list_max(args, line):
    xs = _bi_list_xs("max", args[0], line)
    if not xs:
        raise NovaError(line, "'list.max' kræver en ikke-tom liste")
    if not all(_is_num(x) for x in xs):
        raise NovaError(line, "'list.max' kræver en liste af tal")
    return max(xs)


def _bi_list_keys(args, line):
    bog = args[0]
    if not isinstance(bog, dict):
        raise NovaError(line, "'list.keys' kræver en databog (fx fra json.parse)"
                              f" — fandt {nova_str(bog)}")
    return sorted(bog.keys())


def _bi_list_values(args, line):
    bog = args[0]
    if not isinstance(bog, dict):
        raise NovaError(line, "'list.values' kræver en databog (fx fra json.parse)"
                              f" — fandt {nova_str(bog)}")
    return [bog[k] for k in sorted(bog.keys())]


def _make_json_lib():
    m = ModuleInstance("json", "(standardbibliotek)")
    m.funcs["parse"] = BuiltinFunction("parse", ["text"], _bi_json_parse)
    m.funcs["stringify"] = BuiltinFunction("stringify", ["værdi"], _bi_json_stringify)
    return m


def _make_file_lib():
    m = ModuleInstance("file", "(standardbibliotek)")
    m.funcs["read"] = BuiltinFunction("read", ["sti"], _bi_file_read)
    m.funcs["exists"] = BuiltinFunction("exists", ["sti"], _bi_file_exists)
    m.funcs["write"] = BuiltinFunction("write", ["sti", "tekst"], _bi_file_write)
    return m


def _make_random_lib():
    m = ModuleInstance("random", "(standardbibliotek)")
    m.funcs["between"] = BuiltinFunction("between", ["fra", "til"], _bi_random_between)
    m.funcs["pick"] = BuiltinFunction("pick", ["liste"], _bi_random_pick)
    return m


def _make_time_lib():
    m = ModuleInstance("time", "(standardbibliotek)")
    m.funcs["now"] = BuiltinFunction("now", [], _bi_time_now)
    return m


def _make_math_lib():
    m = ModuleInstance("math", "(standardbibliotek)")
    m.funcs["sqrt"] = BuiltinFunction("sqrt", ["tal"], _bi_math_sqrt)
    m.funcs["round"] = BuiltinFunction("round", ["tal"], _bi_math_round)
    return m


def _make_text_lib():
    m = ModuleInstance("text", "(standardbibliotek)")
    m.funcs["upper"] = BuiltinFunction("upper", ["tekst"], _bi_text_upper)
    m.funcs["lower"] = BuiltinFunction("lower", ["tekst"], _bi_text_lower)
    m.funcs["trim"] = BuiltinFunction("trim", ["tekst"], _bi_text_trim)
    m.funcs["split"] = BuiltinFunction("split", ["tekst", "adskiller"], _bi_text_split)
    m.funcs["join"] = BuiltinFunction("join", ["liste", "adskiller"], _bi_text_join)
    m.funcs["replace"] = BuiltinFunction("replace", ["tekst", "fra", "til"], _bi_text_replace)
    m.funcs["length"] = BuiltinFunction("length", ["tekst"], _bi_text_length)
    m.funcs["contains"] = BuiltinFunction("contains", ["tekst", "søgning"], _bi_text_contains)
    m.funcs["at"] = BuiltinFunction("at", ["tekst", "plads"], _bi_text_at)
    m.funcs["slice"] = BuiltinFunction("slice", ["tekst", "fra", "til"], _bi_text_slice)
    return m


def _make_list_lib():
    m = ModuleInstance("list", "(standardbibliotek)")
    m.funcs["sort"] = BuiltinFunction("sort", ["liste"], _bi_list_sort)
    m.funcs["reverse"] = BuiltinFunction("reverse", ["liste"], _bi_list_reverse)
    m.funcs["min"] = BuiltinFunction("min", ["liste"], _bi_list_min)
    m.funcs["max"] = BuiltinFunction("max", ["liste"], _bi_list_max)
    m.funcs["keys"] = BuiltinFunction("keys", ["databog"], _bi_list_keys)
    m.funcs["values"] = BuiltinFunction("values", ["databog"], _bi_list_values)
    return m


STDLIB_FACTORIES = {
    "json": _make_json_lib,
    "file": _make_file_lib,
    "random": _make_random_lib,
    "time": _make_time_lib,
    "math": _make_math_lib,
    "text": _make_text_lib,
    "list": _make_list_lib,
}


class Interp:
    def __init__(self, seed=None, stdin=None, stdout=None, root_dir=None):
        self.globals = Scope()
        self.funcs = {}
        self.things = {}
        self.mains = []
        self.tracked = set()
        self.history = {}   # name -> [snapshots]
        self.redo_stack = {}  # name -> [snapshots]
        self._ensure_frames = []  # stack af [(expr, line)] — udskydes til funktionsafslutning
        self._modules = {}        # abspath -> ModuleInstance (C05: idempotent import)
        self._import_stack = []   # [(abspath, filnavn)] — cirkulær import-detektion
        self._stdlib = {}         # navn -> ModuleInstance (B03: use standard X)
        self._cur_dir = root_dir if root_dir else os.getcwd()  # relativ import-basis
        if seed is not None:
            _random.seed(seed)
        self.stdin = stdin if stdin is not None else sys.stdin
        self.stdout = stdout if stdout is not None else sys.stdout

    # ---------------- output / input ----------------
    def out(self, text):
        self.stdout.write(text)
        self.stdout.flush()

    def readline(self):
        line = self.stdin.readline()
        if line == "":
            raise ExitSignal()  # stdin udmagt — afslut pænt i stedet for at snurre
        return line.rstrip("\r\n")

    def ask(self, prompt_val, line):
        self.out(nova_str(prompt_val))
        return self.readline()

    # ---------------- program ----------------
    def run(self, stmts):
        for st in stmts:
            if isinstance(st, FuncDef):
                self.funcs[st.name] = Function(st.name, st.params, st.body)
            elif isinstance(st, ThingDef):
                self.things[st.name] = st
            elif isinstance(st, WhenProgramStarts):
                self.mains.append(st)
        for st in stmts:
            if isinstance(st, (FuncDef, ThingDef, WhenProgramStarts)):
                continue
            self.exec_stmt(st, self.globals)
        for m in self.mains:
            self.exec_block(m.body, self.globals)

    # ---------------- statements ----------------
    def exec_block(self, block, scope):
        for st in block.stmts:
            self.exec_stmt(st, scope)

    def exec_stmt(self, st, scope):
        t = type(st).__name__

        if t == "Say":
            parts = [nova_str(self.eval(e, scope)) for e in st.exprs]
            self.out(" ".join(parts) + ("\n" if st.newline else ""))
            return
        if t == "ExprStmt":
            self.eval(st.expr, scope)
            return
        if t == "Assign":
            val = self.eval(st.expr, scope)
            tgt = st.target
            tn = type(tgt).__name__
            if tn == "Var":
                if tgt.name in self.tracked:
                    self._snapshot(tgt.name, scope)
                scope.set(tgt.name, val)
            elif tn == "Field":
                obj = self.eval(tgt.obj, scope)
                if not isinstance(obj, ThingInstance):
                    raise NovaError(st.line, "kan kun sætte felter på en thing")
                obj.fields[tgt.name] = val
            else:
                raise NovaError(st.line, "ugyldigt tildelingsmål")
            return
        if t == "AddTo":
            cur = scope.get(st.name, st.line)
            val = self.eval(st.expr, scope)
            if st.name in self.tracked:
                self._snapshot(st.name, scope)
            if isinstance(cur, list):
                cur.append(val)
            elif _is_num(cur) and _is_num(val):
                scope.set(st.name, cur + val)
            else:
                raise NovaError(st.line, f"'add ... to {st.name}' kræver en liste eller et tal")
            return
        if t == "TakeFrom":
            cur = scope.get(st.name, st.line)
            val = self.eval(st.expr, scope)
            if st.name in self.tracked:
                self._snapshot(st.name, scope)
            if isinstance(cur, list):
                if val in cur:
                    cur.remove(val)
            elif _is_num(cur) and _is_num(val):
                scope.set(st.name, cur - val)
            else:
                raise NovaError(st.line, f"'take ... from {st.name}' kræver en liste eller et tal")
            return
        if t == "If":
            for cond, body in st.branches:
                if self.truth(self.eval(cond, scope), cond):
                    self.exec_block(body, scope)
                    return
            if st.otherwise is not None:
                self.exec_block(st.otherwise, scope)
            return
        if t == "RepeatForever":
            while True:
                try:
                    self.exec_block(st.body, scope)
                except BreakSignal:
                    break
                except ContinueSignal:
                    continue
            return
        if t == "RepeatUntil":
            while True:
                if self.truth(self.eval(st.cond, scope), st.cond):
                    break
                try:
                    self.exec_block(st.body, scope)
                except BreakSignal:
                    break
                except ContinueSignal:
                    continue
            return
        if t == "RepeatWhile":
            while self.truth(self.eval(st.cond, scope), st.cond):
                try:
                    self.exec_block(st.body, scope)
                except BreakSignal:
                    break
                except ContinueSignal:
                    continue
            return
        if t == "RepeatTimes":
            n = self.eval(st.count, scope)
            if not _is_num(n):
                raise NovaError(st.line, "'repeat N times' kræver et tal")
            for _ in range(int(n)):
                try:
                    self.exec_block(st.body, scope)
                except BreakSignal:
                    break
                except ContinueSignal:
                    continue
            return
        if t == "RepeatEach":
            seq = self.eval(st.iterable, scope)
            if not isinstance(seq, (list, str)):
                raise NovaError(st.line, "'repeat for each' kræver en liste eller tekst")
            for item in list(seq):
                scope.set(st.var, item)
                try:
                    self.exec_block(st.body, scope)
                except BreakSignal:
                    break
                except ContinueSignal:
                    continue
            return
        if t == "RepeatCounting":
            a = self.eval(st.start, scope)
            b = self.eval(st.end, scope)
            if not (_is_num(a) and _is_num(b)):
                raise NovaError(st.line, "'repeat with i from A to B' kræver tal")
            for i in range(int(a), int(b) + 1):
                scope.set(st.var, i)
                try:
                    self.exec_block(st.body, scope)
                except BreakSignal:
                    break
                except ContinueSignal:
                    continue
            return
        if t == "BreakStmt":
            raise BreakSignal()
        if t == "ContinueStmt":
            raise ContinueSignal()
        if t == "StopProgram":
            raise ExitSignal()
        if t == "PauseProgram":
            self.readline()
            return
        if t == "Check":
            subj = self.eval(st.subject, scope)
            for kind, val, neg, body in st.arms:
                if self._pat_match(subj, kind, val, neg, scope):
                    self.exec_block(body, scope)
                    return
            if st.otherwise is not None:
                self.exec_block(st.otherwise, scope)
            return
        if t == "TryStmt":
            try:
                self.exec_block(st.body, scope)
            except NovaError as e:
                if st.handler is None:
                    return
                if st.errname:
                    scope.set(st.errname, e.msg)
                self.exec_block(st.handler, scope)
            return
        if t == "FuncDef":
            self.funcs[st.name] = Function(st.name, st.params, st.body)
            return
        if t == "ThingDef":
            self.things[st.name] = st
            return
        if t == "ReturnStmt":
            raise ReturnSignal(self.eval(st.expr, scope) if st.expr is not None else NOTHING)
        if t == "WaitStmt":
            amt = self.eval(st.amount, scope)
            mult = {"seconds": 1.0, "minutes": 60.0, "hours": 3600.0,
                    "milliseconds": 0.001}[st.unit]
            time.sleep(float(amt) * mult)
            return
        if t == "UseLib":
            name = self._stdlib_name(st)
            if name not in self._stdlib:
                self._stdlib[name] = STDLIB_FACTORIES[name]()
            scope.set(name, self._stdlib[name])
            return
        if t == "UseModule":
            inst = self._load_module(st.path, st.name, st.line)
            scope.set(st.name, inst)
            return
        if t == "TrackStmt":
            self.tracked.add(st.name)
            self.history.setdefault(st.name, [])
            self.redo_stack.setdefault(st.name, [])
            return
        if t == "UndoStmt":
            hist = self.history.get(st.name, [])
            redo = self.redo_stack.get(st.name, [])
            if st.redo:
                if redo:
                    snap = redo.pop()
                    self._push_history(st.name, scope.get(st.name, st.line))
                    scope.set(st.name, snap)
                else:
                    raise NovaError(st.line, f"der er ingenting at redo for '{st.name}'")
            else:
                if hist:
                    snap = hist.pop()
                    redo.append(scope.get(st.name, st.line))
                    scope.set(st.name, snap)
                else:
                    raise NovaError(st.line, f"der er ingen ændringer at undo for '{st.name}'")
            return
        if t == "Contract":
            if st.kind == "ensures" and self._ensure_frames:
                # post-betingelse: udskydes til funktionen afslutter
                self._ensure_frames[-1].append((st.expr, st.line))
                return
            ok = self.truth(self.eval(st.expr, scope), st.expr)
            if not ok:
                kind = "requires" if st.kind == "requires" else "ensures"
                raise NovaError(st.line, f"{kind}-kontrakt fejlede — betingelsen "
                                         f"'{kind}' på denne linje var ikke sand")
            return
        if t == "RemoveStmt":
            target = st.expr
            if not isinstance(target, ItemAt):
                raise NovaError(st.line, "'remove' understøtter: remove item N of LISTE")
            lst = self.eval(target.e, scope)
            idx = self.eval(target.idx, scope)
            if not isinstance(lst, list) or not _is_num(idx):
                raise NovaError(st.line, "'remove item N of LISTE' kræver liste og tal")
            i = int(idx)
            if i < 1 or i > len(lst):
                raise NovaError(st.line, f"item {i} findes ikke (listen har {len(lst)} ting)")
            del lst[i - 1]
            return
        if t == "StoreJson":
            val = to_plain(self.eval(st.value, scope))
            path = nova_str(self.eval(st.path, scope))
            try:
                with open(path, "w", encoding="utf-8") as f:
                    _json.dump(val, f, ensure_ascii=False, indent=2)
            except OSError as err:
                raise NovaError(st.line, f"kan ikke gemme til '{path}': {err}")
            return
        if t == "WhenProgramStarts":
            return  # køres til sidst af run()
        raise NovaError(getattr(st, "line", 0), f"ukendt statement {t}")

    def _stdlib_name(self, st):
        """B03: 'use [the] standard NAVN [library]' → NAVN, med venlige fejl."""
        ws = st.text.lower().split()
        if ws and ws[0] == "the":
            ws = ws[1:]
        if not ws or ws.pop(0) != "standard":
            raise NovaError(st.line, f"ukendt 'use'-form: '{st.text}' — skriv: "
                                     "use the standard <navn> library")
        if ws and ws[-1] == "library":
            ws = ws[:-1]
        if len(ws) != 1:
            raise NovaError(st.line, f"ukendt 'use'-form: '{st.text}' — skriv: "
                                     "use the standard <navn> library")
        name = ws[0]
        if name not in STDLIB_FACTORIES:
            raise NovaError(st.line, f"ukendt standardbibliotek '{name}' — "
                                     f"tilgængelige biblioteker: "
                                     f"{', '.join(sorted(STDLIB_FACTORIES))}")
        return name

    def _load_module(self, path_src, bind_name, line):
        """C05: indlæs (eller genbrug) et modul. Cirkulær import = venlig fejl."""
        ap = os.path.abspath(os.path.join(self._cur_dir, path_src))
        if any(p == ap for p, _ in self._import_stack):
            chain = " → ".join(n for _, n in self._import_stack) + f" → {path_src}"
            raise NovaError(line, f"cirkulær import: {chain} — moduler kan ikke "
                                  "importere hinanden i ring; bryd kæden ved at "
                                  "flytte det fælles ud i en tredje fil")
        if ap in self._modules:
            return self._modules[ap]
        try:
            with open(ap, "r", encoding="utf-8") as f:
                src = f.read()
        except OSError:
            raise NovaError(line, f"modul-filen '{path_src}' findes ikke "
                                  f"(søgt i '{self._cur_dir}') — tjek stien og filnavnet")
        try:
            stmts = parse_source(src)
        except NovaLexError as e:
            e.msg += f" (i modulet '{path_src}')"
            raise
        except NovaParseError as e:
            e.msg += f" (i modulet '{path_src}')"
            raise
        inst = ModuleInstance(bind_name, path_src)
        self._modules[ap] = inst
        self._import_stack.append((ap, path_src))
        prev_dir = self._cur_dir
        self._cur_dir = os.path.dirname(ap)
        try:
            for st in stmts:
                if isinstance(st, FuncDef):
                    inst.funcs[st.name] = Function(st.name, st.params, st.body)
                elif isinstance(st, ThingDef):
                    inst.things[st.name] = st
                elif isinstance(st, WhenProgramStarts):
                    raise NovaError(
                        st.line,
                        "et modul må ikke indeholde 'when the program starts' — "
                        "flyt program-starten til hovedprogrammet")
            for st in stmts:
                if isinstance(st, (FuncDef, ThingDef, WhenProgramStarts)):
                    continue
                self.exec_stmt(st, inst.scope)
        finally:
            self._cur_dir = prev_dir
            self._import_stack.pop()
        return inst

    def _pat_match(self, subj, kind, val, neg, scope):
        if kind == "isnum":
            r = self._is_number_value(subj)
            return (not r) if neg else r
        if kind == "isempty":
            r = (subj is NOTHING) or (hasattr(subj, "__len__") and len(subj) == 0)
            return (not r) if neg else r
        v = self.eval(val, scope)
        if kind == "eq":
            r = subj == v
        elif kind == "startswith":
            r = isinstance(subj, str) and subj.startswith(nova_str(v))
        elif kind == "endswith":
            r = isinstance(subj, str) and subj.endswith(nova_str(v))
        elif kind == "contains":
            r = v in subj if hasattr(subj, "__contains__") else False
        else:
            r = False
        return (not r) if neg else r

    def _snapshot(self, name, scope):
        if name not in self.tracked:
            return
        if not scope.has(name):
            return  # første tildeling — der er ingen tidligere værdi at gemme
        hist = self.history.setdefault(name, [])
        self.redo_stack.setdefault(name, []).clear()
        hist.append(copy.deepcopy(scope.get(name, 0)))

    def _push_history(self, name, value):
        self.history.setdefault(name, []).append(copy.deepcopy(value))

    def _is_number_value(self, v):
        if _is_num(v):
            return True
        if isinstance(v, str):
            try:
                int(v)
                return True
            except ValueError:
                try:
                    float(v)
                    return True
                except ValueError:
                    return False
        return False

    def truth(self, v, node=None):
        if isinstance(v, bool):
            return v
        line = getattr(node, "line", 0) if node is not None else 0
        raise NovaError(line, "en betingelse skal være true eller false "
                              "(brug sammenligninger som 'is greater than')")

    # ---------------- udtryk ----------------
    def eval(self, e, scope):
        t = type(e).__name__

        if t == "Lit":
            return e.value
        if t == "StrLit":
            return self.eval_string(e.raw, scope)
        if t == "EmptyListE":
            return []
        if t == "ListLit":
            return [self.eval(x, scope) for x in e.items]
        if t == "Var":
            return scope.get(e.name, e.line)
        if t == "Field":
            obj = self.eval(e.obj, scope)
            if isinstance(obj, ThingInstance):
                if e.name in obj.fields:
                    return obj.fields[e.name]
                self._field_err(obj, e)
            if isinstance(obj, dict):
                if e.name in obj:
                    return obj[e.name]
                raise NovaError(e.line, f"databogen har ikke nøglen '{e.name}'")
            if isinstance(obj, ModuleInstance):
                if e.name in obj.funcs:
                    return obj.funcs[e.name]
                if e.name in obj.scope.vars:
                    return obj.scope.vars[e.name]
                known = sorted(set(obj.funcs) | set(obj.scope.vars))
                raise NovaError(e.line, f"modulet '{obj.path}' har ikke '{e.name}'"
                                        f"{_suggest(e.name, known)}"
                                        f" — gyldige navne: {', '.join(known) or '(ingen)'}")
            if obj is NOTHING:
                raise NothingSignal(
                    e.line,
                    f"kan ikke læse feltet '{e.name}' fra nothing — tilføj '?' "
                    f"hvis udtrykket må være nothing (fx: the {e.name} of x?), "
                    f"eller tjek værdien med 'is nothing' først")
            raise NovaError(e.line, f"kan ikke læse feltet '{e.name}' fra {nova_str(obj)}")
        if t == "Bin":
            return self.eval_bin(e, scope)
        if t == "NotE":
            return not self.truth(self.eval(e.e, scope), e.e)
        if t == "Call":
            return self.call(e.name, [self.eval(a, scope) for a in e.args], e.line)
        if t == "ModuleCall":
            base = scope.get(e.mod, e.line)
            if not isinstance(base, ModuleInstance):
                raise NovaError(e.line, f"'{e.mod}' er ikke et modul — punktum-kald "
                                        f"kræver 'the {e.mod}-module in \"fil.nova\"' først")
            fn = base.funcs.get(e.name)
            if fn is None:
                raise NovaError(e.line, f"modulet '{base.path}' har ikke funktionen "
                                        f"'{e.name}'{_suggest(e.name, base.funcs.keys())}"
                                        f" — kald: {e.mod}.{e.name}(...)")
            args = [self.eval(a, scope) for a in e.args]
            if isinstance(fn, BuiltinFunction):
                if len(args) != len(fn.params):
                    raise NovaError(e.line, f"'{e.mod}.{fn.name}' forventer "
                                            f"{len(fn.params)} argument(er), fik {len(args)}"
                                            f" — kald: {e.mod}.{fn.name}({', '.join(fn.params)})")
                return fn.fn(args, e.line)
            return self._invoke(fn, args, e.line, parent=base.scope)
        if t == "NewThing":
            return self.new_thing(e, scope)
        if t == "AskE":
            return self.ask(self.eval(e.prompt, scope), e.line)
        if t == "QuestionE":
            # C03-grænsen: hele-udtryksgift — ét signal gør hele udtrykket nothing
            try:
                return self.eval(e.e, scope)
            except NothingSignal:
                return NOTHING
        if t == "RandomBetween":
            a = self.eval(e.a, scope)
            b = self.eval(e.b, scope)
            if not (_is_num(a) and _is_num(b)):
                raise NovaError(e.line, "'a random number between A and B' kræver tal")
            return _random.randint(int(a), int(b))
        if t == "NumVal":
            v = self.eval(e.e, scope)
            return self._to_number(v)
        if t == "EverythingAfter":
            sep = nova_str(self.eval(e.sep, scope))
            s = self.eval(e.e, scope)
            s = nova_str(s) if not isinstance(s, str) else s
            i = s.find(sep)
            return s[i + len(sep):] if i != -1 else ""
        if t == "CountOf":
            v = self.eval(e.e, scope)
            if hasattr(v, "__len__"):
                return len(v)
            raise NovaError(e.line, f"'how many items are in' kræver en liste eller tekst")
        if t == "ItemAt":
            lst = self.eval(e.e, scope)
            idx = self.eval(e.idx, scope)
            if not hasattr(lst, "__len__"):
                raise NovaError(e.line, "'item N of' kræver en liste")
            if not _is_num(idx):
                raise NovaError(e.line, "'item N of' kræver et tal som indeks")
            i = int(idx)
            if i < 1 or i > len(lst):
                raise NovaError(e.line, f"item {i} findes ikke (der er {len(lst)} ting)"
                                        f" — gyldige numre er 1 til {max(len(lst), 1)}")
            return lst[i - 1]
        if t == "FirstItem":
            lst = self.eval(e.e, scope)
            if not isinstance(lst, list) or not lst:
                return NOTHING
            return lst[0]
        if t == "LastItem":
            lst = self.eval(e.e, scope)
            if not isinstance(lst, list) or not lst:
                return NOTHING
            return lst[-1]
        if t == "IsEmptyE":
            v = self.eval(e.e, scope)
            if v is NOTHING:
                return True
            if hasattr(v, "__len__"):
                return len(v) == 0
            return False
        if t == "HasNoItems":
            v = self.eval(e.e, scope)
            return not hasattr(v, "__len__") or len(v) == 0
        if t == "ExistsE":
            path = nova_str(self.eval(e.e, scope))
            want = e.flag
            return os.path.exists(path) == want
        if t == "IsNumberTest":
            v = self.eval(e.e, scope)
            r = self._is_number_value(v)
            return (not r) if e.negate else r
        if t == "ContentsOf":
            path = nova_str(self.eval(e.e, scope))
            try:
                if e.as_json:
                    with open(path, "r", encoding="utf-8") as f:
                        return _json.load(f)
                with open(path, "r", encoding="utf-8") as f:
                    return f.read()
            except FileNotFoundError:
                raise NovaError(e.line, f"'{path}' findes ikke")
            except IsADirectoryError:
                raise NovaError(e.line, f"'{path}' er en mappe, ikke en fil")
            except UnicodeDecodeError:
                raise NovaError(e.line, f"'{path}' er ikke en UTF-8-tekstfil")
            except _json.JSONDecodeError as err:
                raise NovaError(e.line, f"'{path}' indeholder ugyldig json (linje {err.lineno})")
            except OSError as err:
                raise NovaError(e.line, f"kan ikke læse '{path}': {err}")
        if t == "EveryTurnedInto":
            src = self.eval(e.e, scope)
            thingdef = self.things.get(e.thing)
            if thingdef is None:
                raise NovaError(e.line, f"ukendt thing '{e.thing}'")
            out = []
            for item in src:
                out.append(self._build_thing(thingdef, item if isinstance(item, dict) else {}, scope))
            return out
        raise NovaError(getattr(e, "line", 0), f"ukendt udtryk {t}")

    def _field_err(self, obj, e):
        raise NovaError(e.line, f"{obj.cls} har ikke feltet '{e.name}'"
                                f"{_suggest(e.name, obj.fields.keys())}"
                                f" — gyldige felter: {', '.join(obj.fields.keys()) or '(ingen)'}")

    def _to_number(self, v):
        if _is_num(v):
            return v
        if isinstance(v, str):
            s = v.strip()
            try:
                return int(s)
            except ValueError:
                try:
                    return float(s)
                except ValueError:
                    return NOTHING
        return NOTHING

    def new_thing(self, e, scope):
        thingdef = self.things.get(e.cls)
        if thingdef is None:
            raise NovaError(e.line, f"ukendt thing '{e.cls}'")
        inst = self._build_thing(thingdef, {}, scope)
        for fname, fexpr in e.setters:
            inst.fields[fname] = self.eval(fexpr, scope)
        return inst

    def _build_thing(self, thingdef, data, scope):
        fields = {}
        for fname, dflt in thingdef.fields.items():
            fields[fname] = self.eval(dflt, scope) if dflt is not None else NOTHING
        for k, v in data.items():
            if k in fields:
                fields[k] = v
        return ThingInstance(thingdef.name, fields)

    def call(self, name, args, line):
        fn = self.funcs.get(name)
        if fn is None:
            raise NovaError(line, f"funktionen '{name}' findes ikke"
                                  f"{_suggest(name, self.funcs.keys())}"
                                  " — definér den med 'to <navn> ... done'")
        if len(args) != len(fn.params):
            raise NovaError(line, f"'{name}' forventer {len(fn.params)} "
                                  f"argument(er), fik {len(args)}"
                                  f" — kald: {name} med {', '.join(fn.params) or 'intet'}")
        return self._invoke(fn, args, line)

    def _invoke(self, fn, args, line, parent=None):
        """Kør en Function. parent = scope-fader (modulfunktioner ser modulens
        globals, hovedprogrammets funktioner ser program-globals)."""
        if len(args) != len(fn.params):
            raise NovaError(line, f"'{fn.name}' forventer {len(fn.params)} "
                                  f"argument(er), fik {len(args)}"
                                  f" — kald: {fn.name} med {', '.join(fn.params) or 'intet'}")
        local = Scope(parent=parent if parent is not None else self.globals)
        for pname, aval in zip(fn.params, args):
            local.declare(pname, aval)
        pending = []
        self._ensure_frames.append(pending)
        result = NOTHING
        try:
            self.exec_block(fn.body, local)
        except ReturnSignal as r:
            result = r.value
        finally:
            self._ensure_frames.pop()
        for expr_node, ln in pending:
            if not self.truth(self.eval(expr_node, local), expr_node):
                raise NovaError(ln, f"ensures-kontrakt fejlede i '{fn.name}' — "
                                    "sluttilstanden opfyldte ikke garantien")
        return result

    def eval_bin(self, e, scope):
        op = e.op
        if op == "and":
            return self.truth(self.eval(e.l, scope), e.l) and \
                   self.truth(self.eval(e.r, scope), e.r)
        if op == "or":
            return self.truth(self.eval(e.l, scope), e.l) or \
                   self.truth(self.eval(e.r, scope), e.r)
        l = self.eval(e.l, scope)
        r = self.eval(e.r, scope)
        if op == "eq":
            return l == r
        if op == "ne":
            return l != r
        if op == "contains":
            if isinstance(l, (str, list)):
                return r in l
            raise NovaError(e.line, "'contains' kræver tekst eller liste")
        if op == "startswith":
            return nova_str(l).startswith(nova_str(r))
        if op == "endswith":
            return nova_str(l).endswith(nova_str(r))
        # numeriske/sammenlignende
        if op in ("gt", "lt", "gte", "lte", "plus", "minus", "times", "divided", "mod"):
            if l is NOTHING or r is NOTHING:
                raise NothingSignal(
                    e.line,
                    "kan ikke regne med 'nothing' — tilføj '?' hvis udtrykket må "
                    "være nothing (fx: n = the number value of answer? + 1), "
                    "eller tjek værdien med 'is nothing' først")
        if op == "gt":  return l > r
        if op == "lt":  return l < r
        if op == "gte": return l >= r
        if op == "lte": return l <= r
        if op == "plus":
            if isinstance(l, str) and isinstance(r, str):
                return l + r
            if _is_num(l) and _is_num(r):
                return l + r
            raise NovaError(e.line, f"kan ikke lægge {nova_str(l)} og {nova_str(r)} "
                                    f"sammen — '+' kræver to tal eller to tekster")
        if op == "minus": return l - r
        if op == "times":
            if isinstance(l, str) and _is_num(r):
                return l * int(r)
            return l * r
        if op == "divided":
            if r == 0:
                raise NovaError(e.line, "division med nul — tjek nævneren, "
                                        "eller brug 'if x is 0' først")
            return l / r
        if op == "mod":   return l % r
        raise NovaError(e.line, f"ukendt operator {op}")

    # ---------------- streng-interpolation ----------------
    def eval_string(self, raw, scope):
        out = []
        i, n = 0, len(raw)
        while i < n:
            ch = raw[i]
            if ch == "{":
                j = raw.find("}", i + 1)
                if j == -1:
                    out.append(ch)
                    i += 1
                    continue
                inner = raw[i + 1:j].strip()
                out.append(nova_str(self.eval_text(inner, scope)))
                i = j + 1
                continue
            out.append(ch)
            i += 1
        return "".join(out)

    def eval_text(self, text, scope):
        from nova_lexer import lex
        from nova_parser import Parser
        toks = lex(text)
        p = Parser(toks)
        expr = p.parse_expr()
        p.skip_newlines()
        if p.peek().kind != "EOF":
            raise NovaError(p.peek().line,
                            f"uventet '{p.peek().value}' i {{{text[:30]}}}")
        return self.eval(expr, scope)

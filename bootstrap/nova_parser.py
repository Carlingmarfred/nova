"""Nova bootstrap parser — Natural syntax → AST.

Regler (v0.1 bootstrap):
- Statements afsluttes af newline (eller ';').
- Multi-line if/repeat/check/to-blokke termineres med 'done'.
- Inline-form: `if C then <stmt>` på én linje kræver ingen 'done'.
- Feltadgang ALTID: `the <felt> of <objekt>` (kan kædes).
"""

from dataclasses import dataclass, is_dataclass, fields as dc_fields, replace as dc_replace
from nova_lexer import lex, NovaLexError, Token


# ---------------- AST: statements ----------------

@dataclass
class Say:
    exprs: list; newline: bool = True; line: int = 0

@dataclass
class Assign:
    target: object; expr: object; line: int = 0

@dataclass
class AddTo:
    name: str; expr: object; line: int = 0

@dataclass
class TakeFrom:
    name: str; expr: object; line: int = 0

@dataclass
class If:
    branches: list; otherwise: object; line: int = 0

@dataclass
class Block:
    stmts: list; line: int = 0

@dataclass
class RepeatForever:
    body: object; line: int = 0

@dataclass
class RepeatUntil:
    cond: object; body: object; line: int = 0

@dataclass
class RepeatWhile:
    cond: object; body: object; line: int = 0

@dataclass
class RepeatTimes:
    count: object; body: object; line: int = 0

@dataclass
class RepeatEach:
    var: str; iterable: object; body: object; line: int = 0

@dataclass
class RepeatCounting:
    var: str; start: object; end: object; body: object; line: int = 0

@dataclass
class BreakStmt:
    line: int = 0

@dataclass
class ContinueStmt:
    line: int = 0

@dataclass
class StopProgram:
    line: int = 0

@dataclass
class PauseProgram:
    line: int = 0

@dataclass
class Check:
    subject: object; arms: list; otherwise: object; line: int = 0

@dataclass
class TryStmt:
    body: object; errname: str; handler: object; line: int = 0

@dataclass
class FuncDef:
    name: str; params: list; body: object; line: int = 0

@dataclass
class ThingDef:
    name: str; fields: dict; line: int = 0

@dataclass
class ReturnStmt:
    expr: object; line: int = 0

@dataclass
class WaitStmt:
    amount: object; unit: str; line: int = 0

@dataclass
class UseLib:
    text: str; line: int = 0


@dataclass
class UseModule:
    """C05: `the X-module in "fil.nova"` — binder X-module til en modul-værdi."""
    name: str; path: str; line: int = 0

@dataclass
class TrackStmt:
    name: str; line: int = 0

@dataclass
class UndoStmt:
    name: str; redo: bool; line: int = 0

@dataclass
class Contract:
    kind: str; expr: object; line: int = 0

@dataclass
class RemoveStmt:
    expr: object; line: int = 0

@dataclass
class StoreJson:
    value: object; path: object; line: int = 0

@dataclass
class ExprStmt:
    expr: object; line: int = 0

@dataclass
class WhenProgramStarts:
    body: object; line: int = 0


# ---------------- AST: udtryk ----------------

@dataclass
class Lit:
    value: object; line: int = 0

@dataclass
class StrLit:
    raw: str; line: int = 0

@dataclass
class ListLit:
    items: list; line: int = 0

@dataclass
class EmptyListE:
    line: int = 0

@dataclass
class Var:
    name: str; line: int = 0

@dataclass
class Field:
    obj: object; name: str; line: int = 0

@dataclass
class Bin:
    op: str; l: object; r: object; line: int = 0

@dataclass
class NotE:
    e: object; line: int = 0

@dataclass
class Call:
    name: str; args: list; line: int = 0


@dataclass
class ModuleCall:
    """C05: modul.funktion(args) — navnerums-kald på en importeret modul-værdi."""
    mod: str; name: str; args: list; line: int = 0

@dataclass
class NewThing:
    cls: str; setters: list; line: int = 0

@dataclass
class NumVal:
    e: object; line: int = 0

@dataclass
class EverythingAfter:
    sep: object; e: object; line: int = 0

@dataclass
class CountOf:
    e: object; line: int = 0

@dataclass
class ItemAt:
    idx: object; e: object; line: int = 0

@dataclass
class FirstItem:
    e: object; line: int = 0

@dataclass
class LastItem:
    e: object; line: int = 0

@dataclass
class IsEmptyE:
    e: object; line: int = 0

@dataclass
class HasNoItems:
    e: object; line: int = 0

@dataclass
class ExistsE:
    e: object; flag: bool; line: int = 0

@dataclass
class IsNumberTest:
    e: object; negate: bool; line: int = 0

@dataclass
class RandomBetween:
    a: object; b: object; line: int = 0

@dataclass
class ContentsOf:
    e: object; as_json: bool; line: int = 0

@dataclass
class EveryTurnedInto:
    e: object; thing: str; line: int = 0

@dataclass
class AskE:
    prompt: object; line: int = 0


@dataclass
class QuestionE:
    """OptionalGuard (C03): opstår KUN ved roden af et udtryk der bar en `?`.
    Markere fjernes under parse; hele træet pakkes ét sted
    (specs/error_handling.md §2.1)."""
    e: object; line: int = 0


class NovaParseError(Exception):
    def __init__(self, line, msg, col=None):
        super().__init__(f"linje {line}: {msg}")
        self.line = line
        self.col = col
        self.msg = msg


# Reserverede ord i Natural-skinnen (specs/syntax/lexical.md §Reserverede ord).
# Kun ord der gør programmet uparselbart — alt andet er frit.
RESERVED_WORDS = frozenset({
    # sætnings-startere
    "say", "write", "if", "unless", "repeat", "stop", "skip", "go", "set",
    "add", "take", "remove", "check", "try", "to", "use", "wait", "pause",
    "track", "undo", "redo", "exit", "when", "requires", "ensures",
    "give", "return", "store",
    # struktur & konnektorer
    "then", "otherwise", "done", "is", "and", "or", "not",
    "the", "of", "in", "from", "with", "a", "an",
    # værdier
    "true", "false", "nothing", "none", "null",
    # indbyggede udtryks-hoveder
    "ask", "every", "everything", "item", "how", "many",
})


class Parser:
    def __init__(self, toks):
        self.toks = toks
        self.pos = 0

    # ---- token-hjælpere ----
    def peek(self, k=0):
        j = min(self.pos + k, len(self.toks) - 1)
        return self.toks[j]

    def next(self):
        t = self.toks[self.pos]
        if t.kind != "EOF":
            self.pos += 1
        return t

    def at_word(self, *words):
        t = self.peek()
        return t.kind == "WORD" and t.value.lower() in words

    def at_kind(self, kind):
        return self.peek().kind == kind

    def at_word_ahead(self, k, *words):
        t = self.peek(k)
        return t.kind == "WORD" and t.value.lower() in words

    def eat_word(self, *words):
        if self.at_word(*words):
            return self.next().value.lower()
        return None

    def expect_word(self, *words):
        if not self.at_word(*words):
            t = self.peek()
            raise NovaParseError(t.line, f"forventede '{'/'.join(words)}' men fandt '{t.value}'"
                                         " — tjek ordlyden i sætningen", t.col)
        return self.next().value.lower()

    def skip_newlines(self):
        while self.peek().kind == "NEWLINE":
            self.next()

    def at_eol(self):
        return self.peek().kind in ("NEWLINE", "EOF")

    def expect_eol(self):
        """Statement-slut: NEWLINE/EOF efterlades til blok-løkken."""
        if not self.at_eol():
            t = self.peek()
            raise NovaParseError(t.line, f"uventet '{t.value}' — forventede linjeslut"
                                         " (én sætning pr. linje)", t.col)

    def err(self, msg):
        t = self.peek()
        raise NovaParseError(t.line, msg, t.col)

    def check_reserved(self, tok, what="navn"):
        if tok.kind == "WORD" and tok.value.lower() in RESERVED_WORDS:
            raise NovaParseError(tok.line,
                                 f"'{tok.value}' er et reserveret ord og kan ikke "
                                 f"bruges som {what} — vælg et andet navn", tok.col)

    def expect_name(self, what="navn"):
        """Forvent et WORD-token og reserver-check det."""
        t = self.peek()
        if t.kind != "WORD":
            raise NovaParseError(t.line, f"forventede et {what}, fandt '{t.value}'", t.col)
        self.next()
        self.check_reserved(t, what)
        return t.value

    # ---------------- program ----------------
    def parse_program(self):
        stmts = []
        self.skip_newlines()
        while self.peek().kind != "EOF":
            stmts.append(self.parse_statement())
            self.skip_newlines()
        return stmts

    # ---------------- statements ----------------
    def parse_statement(self):
        t = self.peek()
        if t.kind != "WORD":
            raise NovaParseError(t.line, f"uventet '{t.value}' — forventede en sætning "
                                         "(fx: say ... / set ... to ... / repeat ...) "
                                         "eller en erklæring som 'x is 5'", t.col)
        w = t.value.lower()

        if w == "use":                                   return self.p_use()
        if w in ("say", "write"):                        return self.p_say()
        if w in ("if", "unless"):                        return self.p_if()
        if w == "repeat":                                return self.p_repeat()
        if w == "stop":                                  return self.p_stop()
        if w == "skip":
            self.next(); self.expect_word("this"); self.expect_word("one"); self.expect_eol()
            return ContinueStmt(line=t.line)
        if w == "go":
            self.next(); self.expect_word("to"); self.expect_word("next"); self.expect_word("turn"); self.expect_eol()
            return ContinueStmt(line=t.line)
        if w == "set":                                   return self.p_set()
        if w == "add":                                   return self.p_addtake("to", AddTo)
        if w == "take":                                  return self.p_addtake("from", TakeFrom)
        if w == "remove":
            self.next(); e = self.parse_expr(); self.expect_eol()
            return RemoveStmt(e, line=t.line)
        if w == "check":                                 return self.p_check()
        if w == "try":                                   return self.p_try()
        if w == "to":                                    return self.p_funcdef()
        if w in ("a", "an") and self.at_word_ahead(2, "is") and self.at_word_ahead(4, "thing"):
            return self.p_thingdef()
        if w == "wait":                                  return self.p_wait()
        if w == "pause":
            self.next(); self.expect_word("the"); self.expect_word("program"); self.expect_eol()
            return PauseProgram(line=t.line)
        if w == "track":
            self.next(); self.eat_word("the")
            name = self.expect_name("variabelnavn")
            self.expect_eol()
            return TrackStmt(name, line=t.line)
        if w in ("undo", "redo"):
            redo = (w == "redo")
            self.next(); self.expect_word("the"); self.expect_word("last")
            self.expect_word("change"); self.expect_word("to"); self.eat_word("the")
            name = self.expect_name("variabelnavn")
            self.expect_eol()
            return UndoStmt(name, redo, line=t.line)
        if w == "exit":
            self.next(); self.expect_eol()
            return StopProgram(line=t.line)
        if w == "when" and self.at_word_ahead(1, "the") and self.at_word_ahead(2, "program"):
            self.next(); self.next(); self.next(); self.expect_word("starts")
            self.expect_eol()
            body = self.p_block({"done"})
            self.expect_word("done"); self.next()
            return WhenProgramStarts(body, line=t.line)
        if w in ("requires", "ensures"):
            self.next(); e = self.parse_expr(); self.expect_eol()
            return Contract(w, e, line=t.line)
        if w in ("give", "return"):
            self.next(); self.eat_word("back")
            e = None
            if not self.at_eol():
                e = self.parse_expr()
            self.expect_eol()
            return ReturnStmt(e, line=t.line)
        if w == "store":                                 return self.p_store()
        if w == "the" and self.peek(1).kind == "WORD" and self.at_word_ahead(2, "in"):
            return self.p_usemodule()

        # NAVN (.felt)* = EXPR  →  kompakt tildeling (samme Assign-node)
        if self.peek().kind == "WORD":
            j = self.pos
            if self.toks[j].kind == "WORD" and self.toks[min(j + 1, len(self.toks) - 1)].kind in ("EQUALS", "DOT"):
                j += 1
                while (j < len(self.toks) - 1
                       and self.toks[j].kind == "DOT"
                       and self.toks[j + 1].kind == "WORD"):
                    j += 2
                if j <= len(self.toks) - 1 and self.toks[j].kind == "EQUALS":
                    first_t = self.next()
                    self.check_reserved(first_t, "variabelnavn")
                    target = Var(first_t.value, first_t.line)
                    while self.at_kind("DOT"):
                        self.next()
                        target = Field(target, self.expect_name("feltnavn"), first_t.line)
                    self.next()  # =
                    e = self.parse_expr()
                    self.expect_eol()
                    return Assign(target, e, line=first_t.line)

        # [the|my] NAME is EXPR  →  deklaration/tildeling
        save = self.pos
        self.eat_word("the")
        self.eat_word("my")
        if self.peek().kind == "WORD" and self.at_word_ahead(1, "is"):
            name_t = self.next()
            self.check_reserved(name_t, "variabelnavn")
            self.next()  # is
            e = self.parse_expr()
            self.expect_eol()
            return Assign(Var(name_t.value, name_t.line), e, line=name_t.line)
        self.pos = save

        # udtryks-sætning (funktionskald)
        e = self.parse_expr()
        self.expect_eol()
        return ExprStmt(e, line=t.line)

    def p_use(self):
        t = self.next()
        parts = []
        while not self.at_eol():
            tok = self.next()
            parts.append(str(tok.value))
        return UseLib(" ".join(parts), line=t.line)

    def p_usemodule(self):
        t = self.next()  # the
        name_t = self.peek()
        if name_t.kind != "WORD" or not str(name_t.value).endswith("-module"):
            raise NovaParseError(
                name_t.line,
                f"et modul-navn skal ende på '-module' — fx: the tools-module in \"tools.nova\" "
                f"(fandt '{name_t.value}')", name_t.col)
        self.next()
        self.expect_word("in")
        pt = self.peek()
        if pt.kind != "STRING":
            raise NovaParseError(
                pt.line, f"forventede en fil-sti i anførselstegn efter 'in' — "
                         f"fx: the {name_t.value} in \"{'tools.nova'}\"", pt.col)
        self.next()
        self.expect_eol()
        return UseModule(name_t.value, pt.value, line=t.line)

    def p_say(self):
        t = self.next()
        newline = (t.value.lower() == "say")
        exprs = [self.parse_expr()]
        while self.at_word("and"):
            self.next()
            exprs.append(self.parse_expr())
        self.expect_eol()
        return Say(exprs, newline, line=t.line)

    # --- if / unless ---
    def p_if(self):
        t = self.next()  # if | unless
        cond = self.parse_expr()
        self.expect_word("then")
        negate = (t.value.lower() == "unless")
        branches = []
        els = None
        used_done = False

        body, used_done = self.p_body({"otherwise", "done"})
        branches.append((NotE(cond, t.line) if negate else cond, body))

        while self.at_word("otherwise"):
            self.next()
            if self.at_word("if"):
                self.next()
                c2 = self.parse_expr()
                self.expect_word("then")
                b2, u2 = self.p_body({"otherwise", "done"})
                used_done = used_done or u2
                branches.append((c2, b2))
                continue
            els, u3 = self.p_body({"done"})
            used_done = used_done or u3
            break

        if used_done:
            self.expect_word("done"); self.next()
        elif self.at_word("done"):
            # tolerance: inline-kæde med ekstra done
            self.next()
        return If(branches, els, line=t.line)

    def p_body(self, stop_words):
        """Efter then/otherwise: newline-blok (stop-ord påkrævet) eller inline."""
        if self.peek().kind == "NEWLINE":
            self.skip_newlines()
            stmts = []
            while True:
                if self.peek().kind == "EOF":
                    self.err(f"blokken mangler afslutning — forventede "
                             f"'{'/'.join(sorted(stop_words))}'; hver blok afsluttes med 'done'")
                if self.peek().kind == "NEWLINE":
                    self.next(); continue
                if self.at_word(*stop_words):
                    return Block(stmts, self.peek().line), True
                stmts.append(self.parse_statement())
        else:
            stmts = [self.parse_statement()]
            while self.peek().kind == "NEWLINE":
                break
            return Block(stmts, self.peek().line), False

    def p_block(self, stop_words):
        self.skip_newlines()
        stmts = []
        while True:
            if self.peek().kind == "EOF":
                self.err("blokken mangler 'done' — hver blok afsluttes med sin egen done")
            if self.peek().kind == "NEWLINE":
                self.next(); continue
            if self.at_word(*stop_words):
                return Block(stmts, self.peek().line)
            stmts.append(self.parse_statement())

    # --- repeat ---
    def p_repeat(self):
        t = self.next()
        if self.eat_word("forever"):
            body = self.p_block({"done"}); self.expect_word("done"); self.next()
            return RepeatForever(body, t.line)
        if self.eat_word("until"):
            cond = self.parse_expr(); self.expect_eol()
            body = self.p_block({"done"}); self.expect_word("done"); self.next()
            return RepeatUntil(cond, body, t.line)
        if self.eat_word("while"):
            cond = self.parse_expr(); self.expect_eol()
            body = self.p_block({"done"}); self.expect_word("done"); self.next()
            return RepeatWhile(cond, body, t.line)
        if self.at_word("each", "for") and (self.at_word("each") or self.at_word_ahead(1, "each")):
            self.eat_word("for"); self.expect_word("each")
            var = self.expect_name("løkkevariabel")
            self.expect_word("in")
            it = self.parse_expr(); self.expect_eol()
            body = self.p_block({"done"}); self.expect_word("done"); self.next()
            return RepeatEach(var, it, body, t.line)
        if self.at_word("with"):
            self.next()
            var = self.expect_name("løkkevariabel")
            self.expect_word("from")
            a = self.parse_term_first_only()
            self.expect_word("to")
            b = self.parse_term_first_only()
            self.expect_eol()
            body = self.p_block({"done"}); self.expect_word("done"); self.next()
            return RepeatCounting(var, a, b, body, t.line)
        count = self.parse_term_first_only()
        self.expect_word("times"); self.expect_eol()
        body = self.p_block({"done"}); self.expect_word("done"); self.next()
        return RepeatTimes(count, body, t.line)

    def p_stop(self):
        t = self.next()
        self.eat_word("the")
        w = self.expect_word("loop", "program")
        self.expect_eol()
        return BreakStmt(t.line) if w == "loop" else StopProgram(t.line)

    def p_set(self):
        t = self.next()
        target = self.parse_lvalue()
        self.expect_word("to")
        e = self.parse_expr()
        self.expect_eol()
        return Assign(target, e, line=t.line)

    def parse_lvalue(self):
        t = self.peek()
        if self.at_word("the"):
            node = self.parse_the_chain()
            if isinstance(node, (Field,)):
                return node
            self.err("'set' med 'the' kræver formen: the <felt> of <objekt> "
                     "(fx: set the text of task to \"hej\")")
        self.eat_word("my")
        if self.peek().kind != "WORD":
            self.err(f"forventede et navn, fandt '{t.value}'")
        nt = self.next()
        self.check_reserved(nt, "variabelnavn")
        if self.at_word("of"):
            self.next()
            obj = self.parse_arith()
            return Field(obj, nt.value, nt.line)
        return Var(nt.value, nt.line)

    def parse_the_chain(self):
        """the F of OBJ  (OBJ må selv være kæde)"""
        t = self.peek()
        self.next()  # the
        if self.peek().kind != "WORD":
            self.err(f"forventede et navn efter 'the', fandt '{t.value}' — "
                     "fx: the text of task")
        head_t = self.next()
        w = head_t.value.lower()
        # indbyggede fraser
        if w == "contents":
            self.eat_word("of")
            src = self.parse_arith()
            as_json = False
            if self.at_word("parsed"):
                self.next(); self.expect_word("as"); self.expect_word("json")
                as_json = True
            return ContentsOf(src, as_json, t.line)
        if w == "first" and self.at_word("item"):
            self.next(); self.expect_word("of")
            return FirstItem(self.parse_arith(), t.line)
        if w == "last" and self.at_word("item"):
            self.next(); self.expect_word("of")
            return LastItem(self.parse_arith(), t.line)
        if w == "number" and self.at_word("value"):
            self.next(); self.expect_word("of")
            # operand på factor-niveau: 'the number value of x? plus 1' skal
            # parse som (nv x?) + 1 — ellers ligger '?'-giften udenom konverteringen
            return NumVal(self.parse_factor(), t.line)
        if w == "length":
            self.eat_word("of")
            return CountOf(self.parse_arith(), t.line)
        if self.at_word("of"):
            self.next()
            obj = self.parse_arith()
            return Field(obj, head_t.value, head_t.line)
        return Var(head_t.value, head_t.line)

    def p_addtake(self, prep, cls):
        t = self.next()
        e = self.parse_arith()
        self.expect_word(prep)
        self.eat_word("the")
        name = self.expect_name("variabelnavn")
        self.expect_eol()
        return cls(name, e, line=t.line)

    # --- check ---
    def p_check(self):
        t = self.next()
        subj = self.parse_expr()
        arms = []
        els = None
        while True:
            if self.peek().kind in ("NEWLINE", "EOF"):
                if self.peek().kind == "EOF":
                    self.err("check mangler 'done' — afslut arm-listen med done")
                self.next(); continue
            if self.at_word("when"):
                wt = self.next()
                self.eat_word("it")
                self.eat_word("is")
                kind, val, neg = self.parse_pattern(wt.line)
                body, _ = self.p_body({"when", "otherwise", "done"})
                arms.append((kind, val, neg, body))
                continue
            if self.at_word("otherwise"):
                self.next()
                els, _ = self.p_body({"when", "done"})
                break
            break
        if self.at_word("done"):
            self.next()
        return Check(subj, arms, els, t.line)

    def parse_pattern(self, line):
        neg = bool(self.eat_word("not"))
        if self.at_word("a", "an"):
            self.next(); self.expect_word("number")
            return ("isnum", None, neg)
        if self.at_word("equal"):
            self.next(); self.expect_word("to")
            return ("eq", self.parse_arith(), neg)
        if self.at_word("the") and self.at_word_ahead(1, "same"):
            self.next(); self.next(); self.expect_word("as")
            return ("eq", self.parse_arith(), neg)
        if self.at_word("starts"):
            self.next(); self.expect_word("with")
            return ("startswith", self.parse_arith(), neg)
        if self.at_word("ends"):
            self.next(); self.expect_word("with")
            return ("endswith", self.parse_arith(), neg)
        if self.at_word("contains"):
            self.next()
            return ("contains", self.parse_arith(), neg)
        if self.at_word("empty"):
            self.next()
            return ("isempty", None, neg)
        return ("eq", self.parse_arith(), neg)

    # --- try ---
    def p_try(self):
        t = self.next()
        body = self.p_block({"if", "done"})
        errname, handler = None, None
        if self.at_word("if"):
            self.next()
            self.expect_word("it"); self.expect_word("fails")
            if self.eat_word("as"):
                errname = self.expect_name("variabelnavn")
            handler = self.p_block({"done"})
        self.expect_word("done"); self.next()
        return TryStmt(body, errname, handler, t.line)

    # --- funktioner og things ---
    def p_funcdef(self):
        t = self.next()
        name = self.expect_name("funktionsnavn")
        params = []
        if self.eat_word("with"):
            while True:
                params.append(self.expect_name("parameternavn"))
                if not self.eat_word("and"):
                    break
        self.expect_eol()
        body = self.p_block({"done"})
        self.expect_word("done"); self.next()
        return FuncDef(name, params, body, t.line)

    def p_thingdef(self):
        t = self.next()  # a/an
        name = self.expect_name("thing-navn")
        self.expect_word("is"); self.expect_word("a"); self.expect_word("thing"); self.expect_word("with")
        fields = {}
        while True:
            self.skip_newlines()
            if self.at_word("done"):
                break
            self.eat_word("a"); self.eat_word("an")
            fname = self.expect_name("feltnavn")
            default = None
            if self.at_word("set"):
                self.next(); self.expect_word("to")
                default = self.parse_expr()
            else:
                self.expect_eol()
            fields[fname] = default
        self.expect_word("done"); self.next()
        return ThingDef(name, fields, t.line)

    def p_wait(self):
        t = self.next()
        amount = self.parse_term_first_only()
        unit_w = self.next().value.lower()
        unit = {"second": "seconds", "seconds": "seconds",
                "minute": "minutes", "minutes": "minutes",
                "hour": "hours", "hours": "hours",
                "ms": "milliseconds", "millisecond": "milliseconds",
                "milliseconds": "milliseconds"}.get(unit_w)
        if unit is None:
            self.err(f"ukendt tidsenhed '{unit_w}' (brug seconds/minutes/hours/milliseconds)")
        self.expect_eol()
        return WaitStmt(amount, unit, t.line)

    def p_store(self):
        t = self.next()
        val = self.parse_arith()
        self.expect_word("in")
        path = self.parse_arith()
        if self.at_word("as"):
            self.next(); self.expect_word("json")
        self.expect_eol()
        return StoreJson(val, path, t.line)

    # ---------------- udtryk ----------------
    def parse_expr(self):
        return wrap_optional(self.parse_or())

    def parse_or(self):
        left = self.parse_and()
        while True:
            if self.at_word("or"):
                t = self.next()
                left = Bin("or", left, self.parse_and(), t.line)
            elif self.at_kind("PIPEPIPE"):
                t = self.next()
                left = Bin("or", left, self.parse_and(), t.line)
            else:
                break
        return left

    def parse_and(self):
        left = self.parse_comparison()
        while True:
            if self.at_word("and"):
                t = self.next()
                left = Bin("and", left, self.parse_comparison(), t.line)
            elif self.at_kind("AMPAMP"):
                t = self.next()
                left = Bin("and", left, self.parse_comparison(), t.line)
            else:
                break
        return left

    def parse_comparison(self):
        left = self.parse_arith()
        while True:
            t = self.peek()
            # kompakte symbol-sammenligninger (samme op-strenge som ord-formerne)
            if t.kind == "EQUALEQUAL":
                self.next()
                left = Bin("eq", left, self.parse_arith(), t.line); continue
            if t.kind == "BANGEQUAL":
                self.next()
                left = Bin("ne", left, self.parse_arith(), t.line); continue
            if t.kind == "LT":
                self.next()
                left = Bin("lt", left, self.parse_arith(), t.line); continue
            if t.kind == "LTE":
                self.next()
                left = Bin("lte", left, self.parse_arith(), t.line); continue
            if t.kind == "GT":
                self.next()
                left = Bin("gt", left, self.parse_arith(), t.line); continue
            if t.kind == "GTE":
                self.next()
                left = Bin("gte", left, self.parse_arith(), t.line); continue
            if t.kind != "WORD":
                break
            w = t.value.lower()
            if w == "is":
                self.next()
                neg = bool(self.eat_word("not"))
                if self.at_word("a", "an"):
                    self.next(); self.expect_word("number")
                    left = IsNumberTest(left, neg, t.line); continue
                if self.eat_word("nothing"):
                    left = Bin("ne" if neg else "eq", left, Lit(None, t.line), t.line); continue
                if self.eat_word("empty"):
                    e = IsEmptyE(left, t.line)
                    left = NotE(e, t.line) if neg else e; continue
                if self.eat_word("true"):
                    left = Bin("ne" if neg else "eq", left, Lit(True, t.line), t.line); continue
                if self.eat_word("false"):
                    left = Bin("ne" if neg else "eq", left, Lit(False, t.line), t.line); continue
                if self.at_word("equal"):
                    self.next(); self.expect_word("to")
                    left = Bin("ne" if neg else "eq", left, self.parse_arith(), t.line); continue
                if self.at_word("the") and self.at_word_ahead(1, "same"):
                    self.next(); self.next(); self.expect_word("as")
                    left = Bin("ne" if neg else "eq", left, self.parse_arith(), t.line); continue
                if self.at_word("greater"):
                    self.next(); self.expect_word("than")
                    left = Bin("lte" if neg else "gt", left, self.parse_arith(), t.line); continue
                if self.at_word("less"):
                    self.next(); self.expect_word("than")
                    left = Bin("gte" if neg else "lt", left, self.parse_arith(), t.line); continue
                if self.at_word("at"):
                    self.next()
                    w2 = self.expect_word("least", "most")
                    if w2 == "least":
                        left = Bin("lt" if neg else "gte", left, self.parse_arith(), t.line)
                    else:
                        left = Bin("gt" if neg else "lte", left, self.parse_arith(), t.line)
                    continue
                left = Bin("ne" if neg else "eq", left, self.parse_arith(), t.line)
                continue
            if w == "contains":
                self.next()
                left = Bin("contains", left, self.parse_arith(), t.line); continue
            if w == "starts":
                self.next(); self.expect_word("with")
                left = Bin("startswith", left, self.parse_arith(), t.line); continue
            if w == "ends":
                self.next(); self.expect_word("with")
                left = Bin("endswith", left, self.parse_arith(), t.line); continue
            if w == "has" and self.at_word_ahead(1, "no"):
                self.next(); self.next(); self.expect_word("items")
                left = HasNoItems(left, t.line); continue
            if w == "exists":
                self.next()
                left = ExistsE(left, True, t.line); continue
            if w == "does" and self.at_word_ahead(1, "not"):
                self.next(); self.next(); self.expect_word("exist")
                left = ExistsE(left, False, t.line); continue
            break
        return left

    def parse_arith(self):
        left = self.parse_term()
        while True:
            if self.at_word("plus") or self.at_kind("PLUS"):
                t = self.next()
                left = Bin("plus", left, self.parse_term(), t.line)
            elif self.at_word("minus") or self.at_kind("MINUS"):
                t = self.next()
                left = Bin("minus", left, self.parse_term(), t.line)
            else:
                break
        return left

    def parse_term(self):
        left = self.parse_factor()
        while True:
            if self.at_word("times") or self.at_kind("STAR"):
                t = self.next()
                left = Bin("times", left, self.parse_factor(), t.line)
            elif self.at_word("multiplied") and self.at_word_ahead(1, "by"):
                t = self.peek()
                self.next(); self.next()
                left = Bin("times", left, self.parse_factor(), t.line)
            elif self.at_word("divided") and self.at_word_ahead(1, "by"):
                t = self.peek()
                self.next(); self.next()
                left = Bin("divided", left, self.parse_factor(), t.line)
            elif self.at_word("over"):
                t = self.next()
                left = Bin("divided", left, self.parse_factor(), t.line)
            elif self.at_word("mod") or self.at_kind("PERCENT"):
                t = self.next()
                left = Bin("mod", left, self.parse_factor(), t.line)
            elif self.at_kind("SLASH"):
                t = self.next()
                left = Bin("divided", left, self.parse_factor(), t.line)
            else:
                break
        return left

    def parse_term_first_only(self):
        """Udtryk hvor 'times' IKKE er operator (repeat-N-times osv.)."""
        left = self.parse_factor()
        while self.at_word("plus", "minus"):
            t = self.next()
            left = Bin(t.value.lower(), left, self.parse_factor(), t.line)
        return left

    def parse_factor(self):
        if self.at_word("not"):
            t = self.next()
            return NotE(self.parse_comparison(), t.line)
        if self.at_kind("BANG"):
            t = self.next()
            return NotE(self.parse_factor(), t.line)
        if self.at_kind("MINUS"):
            t = self.next()
            # unary minus uden ny node: 0 - udtryk (samme semantik som ord-skinnet)
            return Bin("minus", Lit(0, t.line), self.parse_factor(), t.line)
        return self.parse_postfix()

    def parse_postfix(self):
        e = self.parse_primary()
        while True:
            if self.at_kind("DOT"):
                dot_t = self.next()
                name = self.expect_name("feltnavn efter '.'")
                if self.peek().kind == "LPAREN":
                    # C05: navn.funktion(...) = modul-kald (kun på en bar variabel)
                    if not isinstance(e, Var):
                        raise NovaParseError(
                            dot_t.line,
                            "metodekald som .navn(...) kommer i en senere version — "
                            "i denne version er punktum-kald kun til modulfunktioner: "
                            "modul-navn.funktion(...)", dot_t.col)
                    self.next()
                    args = []
                    if self.peek().kind != "RPAREN":
                        args.append(self.parse_arith())
                        while self.peek().kind == "COMMA":
                            self.next()
                            args.append(self.parse_arith())
                    if self.peek().kind != "RPAREN":
                        self.err("mangler ')'")
                    self.next()
                    e = ModuleCall(e.name, name, args, dot_t.line)
                else:
                    e = Field(e, name, dot_t.line)
            elif self.at_kind("QUESTION"):
                t = self.next()
                e = QuestionE(e, t.line)
            else:
                break
        return e

    def parse_primary(self):
        t = self.peek()

        if t.kind == "NUMBER":
            self.next(); return Lit(t.value, t.line)
        if t.kind == "STRING":
            self.next(); return StrLit(t.value, t.line)
        if t.kind == "LPAREN":
            self.next()
            e = self.parse_expr()
            if self.peek().kind != "RPAREN":
                self.err("mangler ')'")
            self.next()
            return e
        if t.kind == "LBRACKET":
            self.next()
            items = []
            while self.peek().kind != "RBRACKET":
                items.append(self.parse_expr())
                if self.peek().kind == "COMMA":
                    self.next()
                elif self.peek().kind != "RBRACKET":
                    self.err("forventede ',' eller ']' i liste")
            self.next()
            return ListLit(items, t.line)
        if t.kind != "WORD":
            self.err(f"uventet '{t.value}' i udtryk")

        w = t.value.lower()
        if w == "true":  self.next(); return Lit(True, t.line)
        if w == "false": self.next(); return Lit(False, t.line)
        if w in ("nothing", "none", "null"):
            self.next(); return Lit(None, t.line)

        if w == "ask":
            self.next()
            prompt = self.parse_arith()
            return AskE(prompt, t.line)

        if w in ("a", "an") and self.at_word_ahead(1, "random"):
            self.next(); self.next(); self.expect_word("number")
            self.expect_word("between")
            a = self.parse_term_first_only()
            self.expect_word("and")
            b = self.parse_term_first_only()
            return RandomBetween(a, b, t.line)

        if w in ("a", "an") and self.at_word_ahead(1, "empty") and self.at_word_ahead(2, "list"):
            self.next(); self.next(); self.next()
            return EmptyListE(t.line)

        if w in ("a", "an") and self.at_word_ahead(1, "new"):
            self.next(); self.next()
            cls_t = self.next()
            self.check_reserved(cls_t, "thing-navn")
            setters = []
            if self.eat_word("with"):
                while True:
                    fname = self.next().value
                    self.expect_word("set"); self.expect_word("to")
                    setters.append((fname, self.parse_arith()))
                    if self.at_word("and") and self.at_word_ahead(2, "set"):
                        self.next(); continue
                    break
            return NewThing(cls_t.value, setters, t.line)

        if w == "how" and self.at_word_ahead(1, "many"):
            self.next(); self.next()
            self.expect_word("items"); self.expect_word("are"); self.expect_word("in")
            return CountOf(self.parse_arith(), t.line)

        if w == "everything":
            self.next()
            self.expect_word("after")
            sep = self.parse_arith()
            self.expect_word("in")
            self.eat_word("the")
            return EverythingAfter(sep, self.parse_arith(), t.line)

        if w == "every" and self.at_word_ahead(1, "item"):
            self.next(); self.next(); self.expect_word("of")
            src = self.parse_arith()
            self.expect_word("turned"); self.expect_word("into")
            self.eat_word("a"); self.eat_word("an")
            thing_t = self.next()
            return EveryTurnedInto(src, thing_t.value, t.line)

        if w == "item":
            self.next()
            idx = self.parse_term_first_only()
            self.expect_word("of")
            return ItemAt(idx, self.parse_arith(), t.line)

        if w == "the":
            return self.parse_the_chain()

        if w == "not":
            tt = self.next()
            return NotE(self.parse_comparison(), tt.line)

        return self.parse_bare()

    def parse_bare(self):
        t = self.peek()
        if t.kind != "WORD":
            self.err(f"forventede et navn, fandt '{t.value}'")
        nt = self.next()
        if self.peek().kind == "LPAREN":
            self.next()
            args = []
            if self.peek().kind != "RPAREN":
                args.append(self.parse_arith())
                while self.peek().kind == "COMMA":
                    self.next()
                    args.append(self.parse_arith())
            if self.peek().kind != "RPAREN":
                self.err("mangler ')'")
            self.next()
            return Call(nt.value, args, nt.line)
        if self.at_word("with"):
            self.next()
            args = [self.parse_arith()]
            while self.at_word("and"):
                self.next()
                args.append(self.parse_arith())
            return Call(nt.value, args, nt.line)
        return Var(nt.value, nt.line)


def _has_question(v):
    """Indeholder træet/delen nogen QuestionE-marker?"""
    if isinstance(v, QuestionE):
        return True
    if is_dataclass(v):
        return any(_has_question(getattr(v, f.name)) for f in dc_fields(v))
    if isinstance(v, (list, tuple)):
        return any(_has_question(x) for x in v)
    if isinstance(v, dict):
        return any(_has_question(x) for x in v.values())
    return False


def _strip_question(e):
    """Fjern ALLE QuestionE-markere (bygger træet om uden dem)."""
    if isinstance(e, QuestionE):
        return _strip_question(e.e)
    if not is_dataclass(e):
        return e
    changed = False
    vals = {}
    for f in dc_fields(e):
        old = getattr(e, f.name)
        new = _strip_question_value(old)
        changed = changed or (new is not old)
        vals[f.name] = new
    return dc_replace(e, **vals) if changed else e


def _strip_question_value(v):
    if isinstance(v, list):
        items = [_strip_question_value(x) for x in v]
        return items if any(a is not b for a, b in zip(items, v)) else v
    if isinstance(v, tuple):
        items = tuple(_strip_question_value(x) for x in v)
        return items if any(a is not b for a, b in zip(items, v)) else v
    if isinstance(v, dict):
        items = {k: _strip_question_value(x) for k, x in v.items()}
        return items if any(items[k] is not x for k, x in v.items()) else v
    return _strip_question(e=v)


def wrap_optional(e):
    """C03 hele-udtryksgift: bar træet en `?`, fjernes markere og HELE træet
    pakkes præcis ét sted (QuestionE ved roden). Uden `?` returneres uændret."""
    if not _has_question(e):
        return e
    stripped = _strip_question(e)
    return QuestionE(stripped, getattr(stripped, "line", 0))


def parse_source(src):
    return Parser(lex(src)).parse_program()

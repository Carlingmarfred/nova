"""Nova bootstrap lexer — Natural-syntax tokens."""

class NovaLexError(Exception):
    def __init__(self, line, msg, col=None):
        super().__init__(f"linje {line}: {msg}")
        self.line = line
        self.col = col
        self.msg = msg


class Token:
    __slots__ = ("kind", "value", "line", "col")

    def __init__(self, kind, value, line, col=None):
        self.kind = kind    # WORD STRING NUMBER NEWLINE LPAREN RPAREN LBRACKET RBRACKET COMMA EOF
        self.value = value
        self.line = line
        self.col = col

    def __repr__(self):
        return f"{self.kind}({self.value!r})"


_ESCAPES = {"n": "\n", "t": "\t", "\\": "\\", '"': '"', "'": "'", "{": "{", "}": "}"}

_SINGLE = {"(": "LPAREN", ")": "RPAREN", "[": "LBRACKET",
           "]": "RBRACKET", ",": "COMMA"}

# kompakt-skin-symboler (C01) — samme semantik som ord-operatorerne
_SKIN_SINGLE = {"=": "EQUALS", "+": "PLUS", "-": "MINUS", "*": "STAR",
                "/": "SLASH", "%": "PERCENT", "<": "LT", ">": "GT",
                "!": "BANG", ".": "DOT", "{": "LBRACE", "}": "RBRACE"}
_SKIN_DOUBLE = {"==": "EQUALEQUAL", "!=": "BANGEQUAL", "<=": "LTE",
                ">=": "GTE", "&&": "AMPAMP", "||": "PIPEPIPE"}


def lex(src):
    if src.startswith("\ufeff"):
        src = src[1:]  # UTF-8 BOM tolereres (specs/syntax/lexical.md §Filformat)
    toks = []
    i, n, line = 0, len(src), 1
    bol = 0  # indeks for start af aktuelle linje (til kolonneberegning)
    while i < n:
        c = src[i]
        if c == "\n":
            toks.append(Token("NEWLINE", "\\n", line, i - bol + 1))
            line += 1
            i += 1
            bol = i
            continue
        if c in " \t\r":
            i += 1
            continue
        if line == 1 and src.startswith("#!", i):
            while i < n and src[i] != "\n":
                i += 1
            continue
        if c == "#" or src.startswith("//", i):
            while i < n and src[i] != "\n":
                i += 1
            continue
        if c in "\"'":
            quote = c
            c0 = i - bol + 1
            i += 1
            buf = []
            while True:
                if i >= n:
                    raise NovaLexError(line, "uafsluttet streng — tilføj det manglende anførselstegn", c0)
                ch = src[i]
                if ch == "\\" and i + 1 < n:
                    nxt = src[i + 1]
                    if nxt not in _ESCAPES:
                        raise NovaLexError(line, f"ugyldigt escape '\\{nxt}' - gyldige: \\n \\t \\\\ \" ' {{ }}", i - bol + 1)
                    buf.append(_ESCAPES[nxt])
                    i += 2
                    continue
                if ch == quote:
                    i += 1
                    break
                if ch == "\n":
                    raise NovaLexError(line, "nylinje inde i streng — brug \\n eller del teksten op", c0)
                buf.append(ch)
                i += 1
            toks.append(Token("STRING", "".join(buf), line, c0))
            continue
        if c.isdigit():
            c0 = i - bol + 1
            j = i
            while j < n and (src[j].isdigit() or src[j] == "_"):
                j += 1
            if j < n and src[j] == "." and j + 1 < n and src[j + 1].isdigit():
                j += 1
                while j < n and (src[j].isdigit() or src[j] == "_"):
                    j += 1
                val = float(src[i:j].replace("_", ""))
            else:
                val = int(src[i:j].replace("_", ""))
            toks.append(Token("NUMBER", val, line, c0))
            i = j
            continue
        if c.isalpha() or c == "_":
            c0 = i - bol + 1
            j = i
            while j < n and (src[j].isalnum() or src[j] in "_-"):
                if src[j] == "-" and j + 1 < n and src[j + 1].isdigit():
                    break  # bindestreg efterfulgt af ciffer = minus-operator
                j += 1
            word = src[i:j].rstrip("-")
            toks.append(Token("WORD", word, line, c0))
            i = j
            continue
        two = src[i:i + 2]
        if two in _SKIN_DOUBLE:
            toks.append(Token(_SKIN_DOUBLE[two], two, line, i - bol + 1))
            i += 2
            continue
        if c in _SKIN_SINGLE:
            toks.append(Token(_SKIN_SINGLE[c], c, line, i - bol + 1)); i += 1; continue
        if c in _SINGLE:
            toks.append(Token(_SINGLE[c], c, line, i - bol + 1)); i += 1; continue
        if c == ";":
            toks.append(Token("NEWLINE", ";", line, i - bol + 1)); i += 1; continue
        raise NovaLexError(line, f"tegnet '{c}' er ikke gyldigt i Nova Natural — "
                                 f"tjek for tastefejl; variabler kan ikke indeholde specialtegn", i - bol + 1)
    toks.append(Token("NEWLINE", "\\n", line, i - bol + 1))
    toks.append(Token("EOF", "", line, i - bol + 1))
    return toks

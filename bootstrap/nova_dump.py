"""Kanonisk AST-dump (golden-format) for Nova bootstrap.

KONTRAKT: Output er deterministisk og linjestabilt for en given kildefil.
Den native M0-parser skal producere byte-kompatibelt output for samme kilde.
Ændr KUN formatet samtidig med alle expected-filer i tests/golden/ og en
notat i project-notes.md.

Format:
  Program (N statements)
    NodeName(line=L)              # hver node på egen linje, 2 mellemrum/niveau
      felt: NodeName(line=L)      # node-felt der peger på node
      felt: skalar-repr           # tal/streng/bool/None via repr()
      felt: [N]                   # liste/tuple af N elementer
        [i] Element(line=L)       # listeelementer indekseres
      felt: {}                    # tom liste
      felt: {N}                   # dict (ThingDef.fields), nøgler i kildesortering
        navn: værdi
"""

from dataclasses import is_dataclass, fields as dc_fields


def dump_program(stmts):
    lines = [f"Program ({len(stmts)} statements)"]
    for st in stmts:
        _emit_node(st, "", lines, 1)
    return "\n".join(lines)


def _emit_node(node, prefix, lines, depth):
    pad = "  " * depth
    name = type(node).__name__
    line = getattr(node, "line", 0)
    lines.append(f"{pad}{prefix}{name}(line={line})")
    for f in dc_fields(node):
        if f.name == "line":
            continue  # allerede i node-overskriften
        _emit_value(f.name, getattr(node, f.name), lines, depth + 1)


def _emit_value(name, v, lines, depth):
    pad = "  " * depth
    if is_dataclass(v):
        _emit_node(v, f"{name}: ", lines, depth)
        return
    if isinstance(v, list):
        if not v:
            lines.append(f"{pad}{name}: []")
            return
        lines.append(f"{pad}{name}: [{len(v)}]")
        for i, item in enumerate(v):
            _emit_item(f"[{i}]", item, lines, depth + 1)
        return
    if isinstance(v, tuple):
        lines.append(f"{pad}{name}: tuple({len(v)})")
        for i, item in enumerate(v):
            _emit_item(f"[{i}]", item, lines, depth + 1)
        return
    if isinstance(v, dict):
        if not v:
            lines.append(f"{pad}{name}: {{}}")
            return
        lines.append(f"{pad}{name}: {{{len(v)} keys}}")
        for k, val in v.items():
            _emit_item(k, val, lines, depth + 1)
        return
    lines.append(f"{pad}{name}: {v!r}")


def _emit_item(label, v, lines, depth):
    pad = "  " * depth
    if is_dataclass(v):
        _emit_node(v, f"{label} ", lines, depth)
        return
    if isinstance(v, tuple):
        lines.append(f"{pad}{label} tuple({len(v)})")
        for i, item in enumerate(v):
            _emit_item(f"[{i}]", item, lines, depth + 1)
        return
    lines.append(f"{pad}{label} {v!r}")

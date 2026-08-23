# Nova — Agent Working Agreement

Nova is a new programming language ("describe the app and it gets built").
This file tells any agent/session how to work in this repo.

## Read first, always

1. `docs/ITERATION_PLAN.md` — single source of truth for priorities and status.
   **⚠ Its Maintenance Rule (§ top) is binding:** after completing ANY item, update
   its status + §11 Changelog (with date) in the same session.
2. `project-notes.md` — implementation invariants and changelog history.
3. `README.md` — decision log for core language decisions.

## Commands

```powershell
python tests/run_tests.py                                   # full suite — must be green
python bootstrap/nova_cli.py run examples/guessing_game.nova --seed 7
python bootstrap/nova_cli.py run examples/todo.nova         # interactive; needs UTF-8 stdin
python bootstrap/nova_cli.py parse examples/todo.nova       # AST dump
python bootstrap/nova_cli.py repl                           # interactive REPL
```

## Hard rules

- Work ONE backlog item at a time, top-down from ITERATION_PLAN §7, skipping blocked.
- Risky items have an excellence contract in ITERATION_PLAN §4.5 — read it BEFORE
  coding; its "Evidence required" list is part of the item's Definition of Done.
- Spec first: language-visible behavior changes go into `specs/*.md` before code.
- Tests first: add failing tests (behavior AND error messages) before implementing.
- Never leave the repo red: `tests/run_tests.py` green + both examples running before
  you finish a session or commit.
- No scope creep: park new ideas in ITERATION_PLAN §10 parking lot instead.
- User-facing error messages are sentences with a fix hint ("That is not a number —
  try again."), never bare Python exceptions. **All diagnostics are English** and
  should migrate toward the catalog in `bootstrap/nova_messages.py`.
- Docs to touch per completed item: ITERATION_PLAN.md (status + changelog),
  project-notes.md (if internals changed), relevant spec, README only for decisions.
- Commit per item: `<ID>: short summary` (e.g. `C01: compact-shorthand lexer skin`).

## Standing owner directives (2026-08-23)

1. **Decouple from Python.** The Python interpreter is a bootstrap/oracle only.
   The native pipeline is written in **Rust** (owner decision 2026-08-23, recorded
   in README decision log). E00 toolchain is installed; E01 CI comes next.
2. **All documentation and all diagnostics are English.** Example *programs* may
   keep Danish UI text for now; new example programs are written in English.
3. Fix known semantic gaps before adding surface features (see plan §12).

## Current state (update this date when it changes)

- v0.15.1-bootstrap (G0 closed; G1 in progress): Python interpreter, **234/234**
  tests green, guessing_game + todo done. Public repo:
  **https://github.com/Carlingmarfred/nova** (Apache-2.0, CI green on windows+ubuntu). Diagnostics fully English.
- Done 2026-08-22: B05 golden dumps · B01 error audit · B02 reserved words ·
  C01 shorthand skin · C02 equivalence pairs · B04 unary minus.
- Done 2026-08-23: C03 Optional (`?`) · C05 modules · B03+C06+C07+C08 stdlib v0 ·
  C09 REPL · C13 memory-model cut · docs-audit · i18n-to-English · semantic-equality
  pinning · mod-zero guard · factor-level phrase binding.
- E00 ✅ + E01 ✅ (CI green). Agreed next-up queue (owner-editable): **D01 → D05 → D06 → C04 → C10 → E02** — see NEXT-UP QUEUE in plan §7.
- Golden dumps: after BEVIDSTE grammar/format changes run
  `python tests/run_tests.py --update-goldens` and review the diff.

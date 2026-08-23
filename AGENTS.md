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
- User-facing error messages are sentences with a fix hint ("Det er ikke et tal — prøv
  igen."), never bare Python exceptions.
- Docs to touch per completed item: ITERATION_PLAN.md (status + changelog),
  project-notes.md (if internals changed), relevant spec, README only for decisions.
- Commit per item: `<ID>: short summary` (e.g. `C01: compact-shorthand lexer skin`).

## Current state (update this date when it changes)

- v0.12-bootstrap (i gang): Python interpreter, **146/146** tests green, guessing_game + todo done.
- Done 2026-08-22: B05 golden dumps · B01 error audit · B02 reserved words ·
  C01 shorthand skin · C02 equivalence pairs · B04 unary minus.
- Done 2026-08-23: C03 Optional (`?` hele-udtryksgift; golden 18; par6; NumVal
  factor-binding; plus type-mismatch-sætning) · C05 modules (navnerum, parentes-kald,
  cirkulær-fejl; golden 19; par7).
- Next up per plan §7: C06–C08 stdlib v0 (B03 stubs først) → C09 REPL.
- Golden dumps: after BEVIDSTE grammar/format changes run
  `python tests/run_tests.py --update-goldens` and review the diff.

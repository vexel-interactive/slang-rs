# Contributing

The full Slang port is developed subsystem by subsystem. Add a small, testable
language feature across its source fixture, grammar, semantic rules, IR, backend,
and documentation. A successful parse must not silently discard semantics.

## Code organization

- The lexer recognizes tokens; syntax modules implement grammar.
- Semantic checking does not emit target code or open source files.
- IR lowering accepts a checked module; backends accept typed IR.
- Filesystem access and external options belong to the compiler API and CLI.
- Each module owns a distinct data model or compilation responsibility.
- Public APIs have rustdoc explaining their invariants and limitations.

The workspace forbids `unsafe`. A future native ABI should isolate any necessary
unsafe operations in a small dedicated crate, with explicit policy changes,
review, and tests.

## Before submitting changes

Run `tools/check.ps1` on Windows, or the commands in `docs/building.md`.
For semantic changes, add a fixture and verify its behavior against upstream.
Update `docs/language.md` and `docs/compatibility.md`. Validate backend artifacts
with an independent target validator.

# Testing

## Workspace checks

`tools/check.ps1` runs rustfmt, Clippy, workspace tests, and rustdoc using Rust
1.99.1. Tests cover precedence, scopes, const/read-only locations, overloads,
macro recursion and operators, include guards, diagnostic locations, IR lowering,
output preservation after a compilation error, switch fallthrough, integer case
constants, do/while exits, cast grammar, and CLI nesting diagnostics. Module
tests cover public/internal access, overload visibility, lexical shadowing,
diamond graphs, reexports, legacy sources, member cycles, macro isolation,
membership errors, and graph limits in the Windows debug CLI. Alias tests cover
forward references, transparent overload types, constructors, resource/struct
aliases, access control, dimension order, nested array parameters, and bounded
alias/array graphs. Constant-buffer tests cover nested read-only locations,
output arguments, imported field visibility, and canonical HLSL element types.
Byte-address tests cover access reflection, load/store widths, atomic output
locations, and invalid method signatures. Independent DXC cases validate each
implemented raw-buffer method.

Examples cover vector addition, control flow, matrix algebra, vertex transforms,
fragment color output, and a multi-file module. With DXC
installed, validate generated HLSL independently:

```text
slangc-rs examples/vector-add.slang -entry main -o vector-add.hlsl
dxc -T cs_6_0 -E main -Fo vector-add.dxil vector-add.hlsl
```

This confirms HLSL validity for DXC. It does not verify GPU execution or complete
language compatibility.

## Differential comparison

`tests/conformance/manifest.json` lists a small positive and negative corpus.
Each source is compiled by both the Rust compiler and a standalone upstream
`slangc`, and their acceptance statuses are compared. Positive HLSL artifacts
are additionally checked with DXC when its executable path is supplied.
Each case may specify a DXC `profile`: `vs_6_0` for vertex and `ps_6_0` for
fragment shaders. The default is `cs_6_0` for compute shaders.

```text
python tools/conformance.py --rust target/debug/slangc-rs.exe --oracle path/to/slangc.exe --dxc path/to/dxc.exe
```

The runner checks only explicitly listed cases. Execution comparisons still
need a separate runner; matching exit codes do not establish semantic
execution equivalence. Reports are written to `out/conformance.json`.

The oracle is published release `v2026.21`; it may differ from the pinned master
source revision. This distinction is recorded in `upstream.lock.json`.
Strict revision parity requires building an oracle from the pinned commit.

## Complete upstream corpus

The inventory contains 6723 upstream test files, not 6723 executed test cases.
Many files are includes, expected outputs, fixtures, or platform-specific inputs.
Before enabling a category, port its upstream test directives, dependencies,
profiles, and comparison methods. Unsupported categories must remain visible
in coverage reports.

## Recorded local validation

[validation.json](validation.json) records the Windows validation run for this
checkpoint. It includes the pinned compiler version, workspace checks, and
differential corpus counts. It is a snapshot of that run, not a promise of
complete Slang compatibility or validation on other operating systems.

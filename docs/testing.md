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

Struct construction tests cover field order, single evaluation, partial and
empty constructors, copies, nested defaults, array fields, aliases, imported
visibility, and generated helper names. CLI tests check excessive aggregate
expansion and recursive field defaults on the Windows debug stack. Differential
fixtures also validate static global initialization and empty structs with DXC.

Initializer-list tests distinguish numeric splats, padding, and matrix rows;
cover flat/nested arrays, returns, assignments, function and resource arguments;
and check overload ambiguity before contextual conversion. Imported field
defaults retain their owning module. Debug CLI tests bound large arrays and
branching defaults as well as recursive syntax. The differential corpus includes
invalid copies, excess inner values, output arguments, and generic inference.

Array constructor tests cover direct and aliased types, dimension order, exact
copies, struct defaults, flat/nested arguments, imported types, and invalid sizes.
Generated HLSL for direct constructors and imported arrays is checked with DXC.
Value-type tests include shared graphs with 128 levels, cached depth limits,
array-mediated cycles, and source-located diagnostics. The Windows debug CLI
also checks a deep shared graph whose first declaration is its outermost type.

Default-argument tests cover per-call evaluation, caller shadowing, prototype
merging, overload ranking, arrays/structs, imported defaults, and output/nontrailing
errors. Recursive and branching function defaults are exercised through the
Windows debug CLI. Differential fixtures validate full target signatures and
declaration-scope helper generation with DXC.

Comma tests check precedence, argument/default separators, void left operands,
result types, loop increments, and rejection of writable or constant uses.
Differential cases also cover comma expressions inside function defaults.

Field-default regression tests distinguish caller-supplied values from defaults
under local variable and function shadowing. Nested structs, brace lists, and
array defaults exercise declaration-scope evaluation. A differential example
also validates the generated helper calls with DXC.

Declaration-group tests cover independent array suffixes, sequential bindings,
const values, field defaults, imported visibility, and loop scope. Differential
examples exercise both numeric and array declarators in `for` initializers;
duplicate names and references after the loop must fail in both compilers.

Binding regressions inspect initializer references under global, parameter, and
nested local shadowing. They also cover const evaluation, declaration groups,
generated prefix collisions, source names in diagnostics, and global reflection.

Discard tests cover fragment HLSL, clip signatures, selected overloads, unused
defaults, imported source locations, and stage propagation through calls,
globals, and aggregate defaults. Long chains and cyclic call graphs check that
requirement propagation does not recurse on the native stack.

Sampling tests cover bias and explicit gradients for all four texture dimensions,
element return types, contextual gradient lists, and invalid receivers and shapes.
Differential fixtures validate fragment bias and compute gradient sampling with DXC.

Output-intrinsic tests exercise writable fields, swizzles, arrays, and resources;
numeric and shape inference; exact output types; const and temporary rejection;
and user functions with intrinsic names. Differential examples check scalar and
vector sincos/frexp, matrix modf, and invalid output arguments with DXC.

Matrix-output tests check row types, exact exponent types, single input evaluation,
helper reuse, defaults, and name collisions. The matrix output example is also
checked directly with DXC. In the installed DXC, upstream's direct matrix sincos
and frexp output crashes the validator; native row lowering avoids those calls.
Module regressions cover builtin isolation and unimported root globals, aliases,
struct fields, and functions.

Examples cover vector addition, control flow, matrix algebra, vertex transforms,
fragment color output, struct construction, and a multi-file module. With DXC
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

Cases normally require both compilers to match `accept`. Documented differences
can set `oracle_accept`, with `oracle_validation_accept` recording whether DXC
should accept its output. Slang 2026.21 misses the stage requirement of a discard
inside an omitted default argument: it accepts a compute shader whose HLSL DXC
rejects. The native compiler diagnoses this before emission. This case requires
both the acceptance difference and DXC's rejection; it is not counted as matching
compiler acceptance. Reports distinguish valid artifacts from rejected artifacts.
Test the comparison rules with `python -m unittest discover -s tools/tests`.

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

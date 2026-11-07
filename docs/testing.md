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

Compute requirement tests cover all six memory barriers, graphics device-only
barriers, used shared globals, local shadowing without module linking, selected
overloads, omitted defaults, and imported source locations. Cyclic graphs merge
independent fragment and compute requirements rather than retaining only the
first one discovered.

Derivative tests check 567 intrinsic/type signatures across all supported floating
scalar, vector, and matrix shapes, plus integer input conversions, helper caching,
single argument evaluation, imports, defaults, and compute quad dimensions.
Profile tests check automatic upgrades, reflected DXC profiles, mismatched stages,
selected entries, and check-only compilation. CLI tests verify warning and
reflection output together.

Matrix selector tests cover both indexing conventions across 96 scalar/matrix
shapes. They check typed scalar results, vector selections, duplicate reads,
write and output rejection, bounds, resource locations, atomics, struct field
names, and imported code. Differential cases also validate the emitted HLSL for
all six scalar component types.

One-component tests cover scalar/vector distinctions, matrix rows, reflection,
explicit and implicit conversions, conditional use, writable components, output
intrinsics, and imported aliases in nested aggregates. Overload tests include
arrays with scalar or one-component vector elements and source declarations that
collide with generated names. Differential fixtures validate all six scalar types
and each matrix shape with one row or column, plus float derivatives, using Slang
and DXC. Native-only fixtures cover floating output functions and half/double
derivatives where the installed validator fails on upstream's HLSL output.

Bit reinterpretation tests check 315 scalar/vector/matrix signatures, explicit
bool/half promotions, identity evaluation, double word output locations, signed
outputs, scalar versus two-component packing, and user functions sharing intrinsic
names. Differential fixtures validate all implemented source shapes and reject
invalid double, arity, width, output-type, and writable-location uses. A native
fixture checks a bitcast-spelled entry through its reflected target name. HLSL
validation does not compare execution or floating-point payloads on a GPU.

Integer bit-operation tests check 60 signatures, unsigned count/position results,
signed reverse results, boolean promotions, and declaration-scope defaults. Invalid
floating, matrix, and arity inputs are rejected. Differential cases validate both
integer signs and each vector width with DXC. Signed search inputs must reach the
target intrinsic without a conversion to unsigned integers.

Predefined macro tests cover physical CRLF lines, nested and multiline initiating
calls, token paste and stringification, escaped file names, included definitions,
module isolation, user overrides, undef, and source-located redefinition warnings.

Thread-constant tests cover typed integer arithmetic, forward dependencies,
transparent aliases, scalar brace initialization, parameter shadowing, imported
public constants that depend on private values, and computed derivative groups.
A 512-declaration dependency chain exercises the worklist. Negative cases cover
runtime dimensions, called functions, size limits, and structural cycles in unused
or inactive initializers. Fractional globals must retain their original arithmetic.
The differential corpus validates these behaviors and forward-global HLSL with DXC.

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

`tools/validate_hlsl.py` validates the fixtures in
[tests/native/manifest.json](../tests/native/manifest.json) with DXC using the
entry name and profile from compiler reflection:

```text
python tools/validate_hlsl.py --rust target/release/slangc-rs.exe --dxc path/to/dxc.exe
```

These native checks cover cases where the installed tools cannot validate an
upstream artifact reliably. They require successful compilation and DXC validation;
a validator crash always fails the case. Results in `out/native-hlsl.json` record
native HLSL validity separately from the differential acceptance corpus.

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
compiler acceptance. The corpus also records graphics group-barrier and shared
atomic uses accepted by Slang 2026.21 whose HLSL DXC rejects. Each difference
requires the expected native diagnostic and the oracle artifact's rejection.
Compute derivatives with invalid quad dimensions are recorded in the same way.
The double dot fixture is accepted by both compilers, but DXC rejects upstream's
double dot DXIL instructions. Its `oracle_validation_accept: false` requires that
rejection alongside valid native component arithmetic. This is an artifact
difference and is counted separately from compiler-acceptance differences.
The optional case `compile_profile` supplies a compiler profile independently of
the DXC `profile`, allowing an automatic upgrade to be checked. `rust_warning`
requires the corresponding warning code in the native compiler's output.
Reports distinguish valid artifacts from rejected artifacts.
Optional `rust_stderr_contains` and `oracle_stderr_contains` lists require every
declared diagnostic fragment from the corresponding compiler. Source-line cases
use them to compare displayed filenames and line numbers, including negative
locations and errors on a continued directive.
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

Local storage regression cases cover conditional first initialization, repeated
calls, shadowing, aggregate defaults, grouped static declarations, loop
initializers, and imported functions. Unbraced declaration cases cover positive
scope behavior and conflicting or escaping bindings. Accepted differential
fixtures are validated with DXC; these checks do not execute the shaders.

Numeric type application cases compare explicit shapes, upstream core defaults,
alias constructors, imported scalar arguments, nested resources, and arrays.
Invalid shape, component, arity, and resource arguments must fail before IR
lowering.

Numeric cast fixtures check scalar splats, vector component conversions, boolean
conversions, matrix truncation, imported component aliases, and incompatible
shapes. Value comparisons using names `vector` and `matrix` remain expressions
and must not be parsed as type applications.

Integer atomic fixtures cover both integer signs and optional output forms,
groupshared variables and arrays, RW structure fields, and RW textures. Negative
fixtures reject ordinary locals, shadowing, read-only locations, constant outputs,
and incorrect comparison arity. The corpus validates shader artifacts and does
not measure concurrency or memory-order behavior on a GPU.

Source-line tests cover physical and displayed coordinates, UTF-8 columns,
LF/CRLF/bare CR, EOF, macro arguments, filename retention, resets, escapes,
signed conversion, and repeated preprocessing. Compiler regressions check
include/import/member resolution, constant and thread-group uses, and locations
from preprocessing, lexing, syntax, semantics, and profile diagnostics. Differential
fixtures compare line conversion, initiating calls, continuations, physical file
resolution, and upstream's rejection of negative location tokens in typed expressions.

Geometric regressions check scalar and vector shapes, integer promotion, mixed
precision, scalar splats, boolean cross-product selection, and single evaluation
of integer helper arguments. Default and imported bindings, generated name
collisions, contextual lists, and explicit double-to-float HLSL conversions are
checked independently. Differential fixtures validate all implemented component
types with DXC and reject invalid dimensions, matrices, boolean distance inputs,
and non-scalar or brace-list refraction indices. These checks do not compare GPU
results or establish full floating-point execution equivalence.
Dot fixtures also cover boolean and integer components, mixed types, scalar
inputs, one-component vectors, contextual lists, default bindings, and double
component reductions. The workspace tests check helper reuse and input side
effects independently of the DXC artifact checks.

[validation.json](validation.json) records the Windows validation run for this
checkpoint. It includes the pinned compiler version, workspace checks, and
differential corpus counts. It is a snapshot of that run, not a promise of
complete Slang compatibility or validation on other operating systems.

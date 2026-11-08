# Testing

## Workspace checks

Numeric literal regressions cover every supported suffix family, selected
overloads, direct decimal-to-float rounding, half ties to even, hex truncation,
subnormals, maximum finite values, infinity, negative zero, malformed spellings,
and literal-to-integer dimensions. Parentheses preserve upstream's restriction
on literal-specific folding. Octal shader values and the preprocessor's separate
32-bit conversion rules have independent checks.
Character tests cover unsigned overload selection, Unicode input and 32-bit
escapes, octal/hex/Unicode forms, empty and multiple values, malformed escapes,
and exact diagnostic spans. Macro expansion, forward dimensions, switch labels,
imported aliases, and default arguments exercise the whole pipeline. The
character corpus transcribes all 55 values from the pinned frontend value test;
the DXIL comparator checks those words and four context results in both
compilers' artifacts.

Wide-integer fixtures transcribe 86 non-pointer boundary and prefix cases from
the pinned integer literal type tests, including additional suffix/context cases.
DXC verifies 40 words from 64-bit arithmetic, casts, signed minima, and shapes.
Five words differ because upstream folds high-bit unsigned comparisons as signed;
both artifacts must match their explicit goldens. Dynamic buffer tests cover
wide vectors, matrices, overloads, constructors, switches, and geometric/math
operations. Reflection and DXC independently verify a 120-byte constant buffer,
including eight-byte component alignment, matrix stride and array tails.

A further fixture compares 24 compiled words from mixed compound assignments,
conditional branches and arithmetic intrinsics. It checks signed and unsigned
scalar, vector and matrix destinations, returned assignment values, narrowing
of floating operands and an index function called exactly once. Floating shift
counts are rejected by both frontends. These words match in both artifacts.

Wide bit-operation fixtures verify 112 compiled words, including counts and
searches across both 32-bit halves, signed negative inputs, zero search results,
64-bit reversal, vector results, declaration defaults, result overload types,
and one evaluation of a postfix argument. A dynamic structured-buffer fixture
also validates all four operations on signed 64-bit vectors through DXC.

Pointer-integer fixtures add 51 literal type cases (including all 11 pointer
cases in the pinned type/minimum tests), 80 ordered scalar/vector/matrix binary
promotions, 25 arithmetic/conversion words, 37 inference/overload words, ten
default/switch/index context words, and 112 bit-operation words. Of these 315
compiled words, 313 match and two have separate exact goldens for upstream's
signed folding of high-bit unsigned comparisons. Pointer templates retain
their own source types while producing 64-bit HLSL storage. Both compilers'
DXC artifacts verify the same 120-byte buffer layout. Native reflection also
verifies its fields and strides; Slang 2026.21 JSON reflection omits pointer
field sizes/offsets despite emitting the correct HLSL. The pinned scalar layout
implementation uses the target pointer size. Other target widths remain unverified.
Mixed scalar/vector inference covers `dot`, `min`, `mul` and conditional
expressions, including 32-bit results requiring input truncation before arithmetic.
Matrix minimum overloads reject scalar arguments in both frontends.

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
Array dimension tests include computed declarations and constructors, forward
and imported constants, lexical shadowing, scalar conversions, cycles, and
runtime rejection. Empty-array regressions check distinct overloads, copies,
defaults, returns, nested dimensions, empty resources and packed buffers,
neighboring uniform offsets, and range overflow avoided by zero dimensions.
DXC verifies the physical buffer extents and offsets independently of source
reflection. Native fixtures also exercise dynamic empty numeric locations and
output arguments; undefined resource indexing reports a bounded diagnostic.
Array-return fixtures cover prototypes, branches, direct return types, overloaded
dimensions and resource families, per-call defaults, globals, imports, matrix
qualifiers, constant-buffer loads, and single evaluation. Both frontends and DXC
check positive fixtures. Shape errors, excess returns, temporary writes, and
array-valued shader entries fail. Native fallthrough errors and upstream's
warning are recorded as an explicit acceptance difference.
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

Register tests cover all implemented resource families, class mismatches, spaces,
decimal normalization, declaration groups, aliases, imports, macros, target-name
rewriting, and reflection schema 7. Resource arrays exercise multiplied counts,
read-only contents, constant-buffer validation, and bounded register ranges.
Nested overlaps produce warnings without hiding errors or failing check-only
compilation. Source remapping remains visible in binding warnings. Differential
fixtures validate the HLSL with DXC; explicit overlaps are accepted with warnings
by both frontends and rejected by DXC. Overflowing indices are documented
compiler-acceptance differences. Ignored annotations on data uniforms, struct
fields, private storage, parameters, and results preserve implicit layout and
do not reserve resource slots. Legacy `c` registers cannot enter the allocator.
Allocator tests cover merged overlaps, adjacent intervals, smaller holes after
array allocation, separate classes/spaces, and exhaustion at the final slot.
Compiler tests verify explicit reservation before automatic assignment, implicit
uniform slots, root-before-import ordering, unused resources, and independent
compilations. Selected differential cases compare normalized resource bindings
from both reflection outputs against declared golden ranges as well as validating
HLSL. They compare class, index, space, and count; uniform byte layout remains
outside these checks.

Direct constant-buffer tests check global field lookup, declaration defaults,
runtime const fields, private static storage, lexical shadowing, module
visibility, member files, empty elements, duplicate buffer names, and invalid
writes or output arguments. Differential fixtures compare the buffers' resource
bindings and field byte layouts. Native DXC buffer definitions independently
verify those layouts using target buffer names from reflection.
Packoffset tests include explicit-first placement, overlaps, decimal
normalization, arrays, unaligned double storage, imports, static members, and
invalid contexts. A crossing vector offset is accepted by both frontends and
rejected by DXC; its rejection is checked explicitly rather than counted as a
valid shader artifact.
Packed struct tests cover constant-buffer parameters, nested values, fixed
arrays, imports, defaults, source reflection, local shadowing without module
linking, and bounded materialization. Physical DXC layout checks ensure that
used buffers survive optimization, including when their values pass through
functions. Separate expected layouts record the upstream HLSL's loss of nested
packing. Valid HLSL with an incorrect physical layout fails the comparison.

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
The overlapping-register fixture sets both `rust_validation_accept` and
`oracle_validation_accept` to false: the frontends succeed with warnings while
DXC rejects both artifacts. Numeric uniform bindings and overflowing register
indices use `oracle_accept: true` with the expected native rejection.
The optional case `compile_profile` supplies a compiler profile independently of
the DXC `profile`, allowing an automatic upgrade to be checked. `rust_warning`
requires the corresponding warning code in the native compiler's output.
Reports distinguish valid artifacts from rejected artifacts.
An optional `dxc_constant_stores` map requires exact uint values by output index
in the compiled DXIL of a single structured UAV. The comparator also decodes
constant `splitDouble`/`extractvalue` instructions. Dynamic, vector, repeated,
or ambiguous stores fail these checks. The rounding and boundary fixtures check
29 stored words against expected IEEE values in both compilers' artifacts;
native fixtures require the same values. This verifies emitted constants,
including subnormals and negative zero, without claiming GPU execution.
An optional `bindings` map specifies expected direct resource ranges by source
name. The runner requests JSON reflection from both compilers, normalizes the
four range fields, and requires every named range to match. Native origin flags
are checked by workspace tests. Reports record the selected binding checks and
their case count. Sources with ambiguous repeated resource names require a
future module-qualified comparison model.
The optional `global_uniform_binding` object checks the shared global uniform
buffer register; `null` requires its absence. The runner compares native JSON
reflection and both emitted HLSL containers. Slang 2026.21's `globalScope.binding`
reports a relative zero even when its HLSL uses `b1`, so the upstream comparison
reads the emitted `globalParams_N` container. These checks require unambiguous
container names in the selected fixtures. They compare register bindings without
claiming byte-layout or JSON-reflection parity. Reports count these cases separately.
An optional `uniform_layouts` map compares selected global and nested field byte
locations with Slang JSON reflection. Paths use source names separated by dots;
`[]` selects the first element without expanding arrays. Entries can require
`offset`, `size`, or both. Layouts are compared only for cases with matching
legacy HLSL rules; native half and single-column matrix layouts are verified
separately against DXC. Reports count these layout cases.
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
Computed dimension regressions cover forward dependencies, wider intermediate
integer arithmetic, lexical shadowing, imported declaration scopes, bool and literal
conversions, casts and constructors, and nested resource applications. Negative
fixtures reject runtime values, dependency cycles, division by zero, and right
angle operators forbidden by the upstream template grammar. A deep dependency
chain checks bounded diagnostics. Selected reflection and physical DXC buffer
layouts verify that dimension expressions become the expected canonical shapes.
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

Global uniform buffer regressions cover explicit `b0` reservations, first-fit gaps,
independent register spaces, aggregates, arrays, imports, private storage exclusion,
generated identifier collisions, `check_only`, and isolated compilation state.
Differential fixtures compare the shared register and direct resource ranges;
DXC validates both compilers' HLSL.

Constant-buffer byte layout tests cover vector register boundaries, nested struct
tail reuse, nested arrays and aliases, resource arrays, root/import order,
`check_only`, private storage, large arrays, shared type graphs, and 64-bit
overflow diagnostics. The native HLSL manifest can additionally specify
`uniform_layouts` and `dxc_layouts`. The runner compares reflection locations and
DXC's buffer definitions, including occupied buffer sizes and selected field
offsets. Those selected DXC cases require unambiguous field names within each
buffer. All 48 float/double/half matrix shapes are checked this way, including
single-column shapes whose legacy DXC packing differs from Slang reflection.

Matrix storage tests cover declaration contexts, qualifiers through forward and
imported aliases, multidimensional arrays, casts, last-modifier selection,
resource element types, overloads, and matching default resource types. CLI
tests compare emitted pragmas and check-only reflection under both defaults.
DXC verifies all 96 float/double/half matrix shapes and both storage orders,
including single-row alignment and reused tail space. Differential cases compare
mixed layouts, row-major aliases, array element locations, and packed matrices.
Structured-buffer copies exercise nested arrays, Load, side-effecting indices,
output arguments, and atomic destinations. Excessive materialization fails with
a bounded diagnostic.

The qualified numeric structured-buffer example records an upstream artifact
difference: Slang 2026.21 emits incompatible storage wrappers for a resource
parameter or Load result, depending on the default order. Both frontends accept
the source; DXC must accept the native artifact and reject the upstream one.
These checks establish shader validity and selected physical layouts, without
claiming GPU execution equivalence. A manifest's `matrix_major` value (`row` or
`column`) applies the same compiler option to both frontends.

Constant-buffer element tests cover all 126 supported numeric types: six scalar
families, vectors of widths one through four, and matrices with one through four
rows and columns. Twelve native DXC cases verify 252 physical and reflected
element layouts under both matrix defaults. Differential fixtures cover numeric
and aggregate values, handle overloads and returns, declaration defaults, nested
struct and matrix arrays, read-only outputs, and void-element index side effects.
Selected bindings and byte locations are compared with upstream reflection;
native DXC buffer definitions independently check physical storage. Large array
reads stay compact, while excessive whole-array copies report `I004`.

Slang 2026.21 emits `ConstantBuffer<float>` for a numeric handle array, which DXC
rejects because it requires a struct element. That fixture requires both
frontends to accept the source, DXC to accept native storage wrappers, and DXC
to reject the upstream artifact. Direct switches on integer constant-buffer
handles are rejected by both frontends; explicitly loaded integer selectors
are accepted. These tests do not compare GPU execution.

[validation.json](validation.json) records the Windows validation run for this
checkpoint. It includes the pinned compiler version, workspace checks, and
differential corpus counts. It is a snapshot of that run, not a promise of
complete Slang compatibility or validation on other operating systems.

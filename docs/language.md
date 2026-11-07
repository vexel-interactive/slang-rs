# Supported language subset

The source language uses Slang syntax. Version 0.1.0 implements a small subset
of upstream features. This document describes its actual boundaries; upstream's
language reference remains the specification for the complete port.

## Preprocessing

Object and function macros support fixed and variadic parameters, nested calls,
and aliases. Directives include `#define`, `#undef`, `#include`, `#pragma once`,
`#if`, `#ifdef`, `#ifndef`, `#elif`, `#else`, `#endif`, `#error`, and `#warning`.
Conditions support `defined`, integer operators, and short-circuit `&&`/`||`.

Include guards, line continuations, `#` stringification, `##` token pasting,
and `__VA_ARGS__` are supported. Arguments for `#` and `##` are used before macro
expansion. Pasted tokens are lexically validated and rescanned. Unused arguments
are not expanded.

Quoted includes search the current file's directory before `-I` paths. Angle
includes search `-I` paths. `MemoryIncludes` resolves exact names from a map.
`__VA_OPT__`, GNU comma elision, predefined macros, `#line`, full redefinition
rules, and macro expansion stacks remain to be ported.

Limits: 64 include levels, 128 macro nesting levels, and 100000 steps per
expansion. `#if` uses signed i64 wrapping arithmetic; complete upstream
signed/unsigned evaluation rules are not implemented.

## Declarations and types

- Functions, prototypes, overloads, and `in`, `out`, `inout` parameters.
- Global and local variables, and simple structures with fields.
- `bool`, `int`, `uint`, `half`, `float`, `double`, and vectors with 2-4 components.
- Scalar names `int32_t`, `uint32_t`, `float32_t`, and `float64_t` resolve to the
  corresponding native scalar types.
- Matrix aliases such as `float2x3`, with 2-4 rows and 2-4 columns.
- Fixed arrays with positive decimal literal dimensions, including nested arrays.
- Global `typedef` and nongeneric `typealias` declarations.
- `StructuredBuffer<T>` and `RWStructuredBuffer<T>` with indexed element access.
- `ConstantBuffer<T>` with a struct element and read-only field access.
- `ByteAddressBuffer` and `RWByteAddressBuffer` with explicit byte-address methods.
- `Texture1D/2D/3D/Cube<T>`, `RWTexture1D/2D/3D<T>`, `SamplerState`, and
  `SamplerComparisonState`. Texture elements are half/float/int/uint scalar or
  vector types; omitting `T` selects `float4`.

Direct `cbuffer` declarations, pointers, 64-bit integers,
and user-defined generics are not implemented.

Modules, imports, reexports, and member files are described in
[Modules and access control](modules.md). Globals, functions, structures, and
structure fields accept `public/internal`. Explicit module declarations use
internal visibility by default; legacy modules export all declarations.

Supported storage modifiers are `const/static/uniform/groupshared` for globals,
`const/static` for locals, and `const/in/out/inout/nointerpolation` for parameters.
Structure fields support semantics and `nointerpolation`. Conflicting
parameter directions and `const out/inout` are rejected. Global values without
`static/groupshared` are read-only; RW buffer elements remain writable.
Global const declarations with initializers require `static`.
Complete modifier rules are not yet implemented. Comma-separated declarations
and default arguments are not supported.

## Type aliases and arrays

Aliases are transparent: they resolve to their underlying types during checking
and do not create distinct overload parameter types. Both forms are supported:

```slang
typedef float Number;
typealias Vector = float2;
typedef Number Row[2];
typealias Grid = Row[3];
```

Aliases may refer to later aliases or structs, and may name resources. Cycles,
unknown targets, and excessively deep alias chains produce `T002`. Public aliases
obey module visibility and cannot expose internal structs. An internal numeric
alias can underlie a public numeric alias because the resolved numeric type is
public. Numeric alias constructors such as `Vector(1.0)` use the resolved target
constructor. The backend emits canonical types rather than alias declarations.

Trailing array declarators use conventional outer-to-inner order:
`typedef int Grid[2][3];` creates a two-element array of three-element arrays.
Leading array suffixes wrap the type in source order:
`typedef int[2][3] Grid;` creates a three-element array of two-element arrays.
The same distinction applies to variable declarations. Reflection uses
outer-to-inner dimensions and HLSL preserves that order in declarations.

Arrays can appear as variables, struct fields, and parameters. Unsized arrays,
symbolic sizes, array constructors, array return types,
and array elements inside resource types are not implemented. Alias chains and
resolved array ranks are each limited to 32. Local/nested aliases, generic
aliases, qualified type references, pointer declarators, and C-style casts using
user alias names remain unported. See [the alias example](../examples/type-aliases.slang).

## Struct construction and field defaults

Named constructors initialize data-only structs in field declaration order:

```slang
struct Pair
{
    float x = 7.0;
    int y = 3;
};

Pair a = Pair(2.0, 4);
Pair b = Pair(2.0); // x = 2, y = 3
Pair c = Pair();    // x = 7, y = 3
Pair d = Pair(a);   // copy; does not evaluate field defaults
```

Omitted fields use their declared defaults. Without a declared default, scalar,
vector, and matrix fields are zero; nested structs and fixed arrays recursively
initialize their fields and elements. Defaults are checked in the struct's
owning module. Transparent aliases use the same constructor. Public structs
accept constructor arguments only for explicitly public fields, including when
called from their own module. Internal fields still receive their defaults.

Declarations without an initializer remain uninitialized and do not invoke
field defaults. User-defined constructors and resource-valued memberwise
construction remain unimplemented. Default
aggregate expansion is limited to 32 levels and 65536 elements. Recursive
constructor defaults report a diagnostic during lowering. See
[struct-construction.slang](../examples/struct-construction.slang).

## Brace initializer lists

`{...}` uses the type of its declaration, assignment destination, return type,
or selected function parameter. Lists support nested data-only structs, fixed
arrays, scalars, vectors, and matrices. Trailing commas and empty lists are valid.

```slang
float values[3] = {1.0, 2.0}; // final element is zero
float grid[2][3] = {{1.0}, {2.0, 3.0, 4.0}};
float3 splat = {2.0};        // all components are 2
float3 partial = {1.0, 2.0}; // final component is zero
float2x2 rows = {1.0, 2.0};  // first row is 1, second row is 2
```

Struct lists first try memberwise initialization, including field defaults and
visibility. When that form does not apply, flat values fill nested fields in
declaration order. Arrays also accept flat lists across dimensions. Explicit
nested lists keep their own boundaries; an excess value in an inner list is
an error. Empty lists use zero values and nested struct defaults. Copy a struct
with its named constructor or ordinary assignment.

User function overloads treat list conversions as having equal cost and select
using their other arguments. Two otherwise matching overloads are ambiguous
even if one target would later reject the list's shape. Resource input arguments
use their method signature; output parameters still require writable locations.
Selected numeric constructor overloads and `dot/min/max/pow/clamp/lerp` support
lists when their other arguments establish a type. A unary intrinsic such as
`sin({1.0})` cannot infer its generic argument from the list.

The current frontend does not implement every upstream constructor overload,
resource-valued aggregate, or generic inference rule. Aggregate normalization
and expanded defaults are bounded to 65536 nodes; excessive nesting and size
produce diagnostics. See
[aggregate-initialization.slang](../examples/aggregate-initialization.slang).

## Expressions and statements

Expressions include numeric and boolean literals, names, scalar, vector, and
matrix and struct constructors, unary and binary operators, assignment, compound assignment,
function calls, fields, swizzles, indexing, conditionals, and increment/decrement.

Statements include blocks, local declarations, expression statements, `return`,
`if/else`, `while`, `for`, `do/while`, `switch`, `break`, and `continue`.
C-style casts support scalar-to-scalar conversions. Use constructors for vector
and matrix values. The comma operator and statement attributes remain to be ported.

The checker resolves nested scopes, validates conversions and scalar conditions,
and tracks writable locations. It rejects writes to const values, read-only
buffers, and swizzles with repeated components. Overload conversions currently
have equal cost after exact matches. Full overload ranking, definite assignment,
general constant evaluation, uniformity analysis, and capability checking are absent.

Syntax recursion is limited to 32 levels and expression trees to 64 levels.
These limits also apply to debug executables on Windows, where the default
thread stack is small. Excessive nesting produces `S001` instead of a stack
overflow. An iterative parser remains necessary for deeper upstream programs.

## Switches and control flow

Switch selectors require scalar `int` or `uint`. Labels support integer
literals, const bindings, unary/binary integer operations, conditional
expressions, and scalar integer conversions. The evaluator tracks 32-bit signed
and unsigned expression types. Duplicate values after selector conversion,
duplicate defaults, and nonconstant labels are rejected. Function calls in
constant expressions and the complete upstream constant evaluator remain absent.
Global const expressions are currently processed in declaration order.

Consecutive labels and fallthrough are preserved. `break` exits the nearest
loop or switch; `continue` requires a surrounding loop, including when nested
inside a switch. Return checking tracks possible fallthrough, return, break,
and continue exits. A do/while body executes at least once; a switch with a
default can satisfy return checking when every entry path returns. This is a
structured analysis rather than a complete CFG or definite-assignment analysis.

## Matrix operations

Matrix aliases specify rows and columns, for example `float2x3`. Constructors
accept one scalar (splat), a matrix with the same dimensions, or scalar/vector
arguments containing exactly `R * C` components. Indexing selects a row vector:
`float2x3 m; m[0]` has type `float3`, and `m[0][1]` selects one component.
Matrix row indices are not statically bounds-checked. Array, vector, matrix, and
structured-buffer indexing requires a scalar `int` or `uint`; upstream permits numeric conversions
with warnings, which this frontend does not yet implement.

Arithmetic operators, including `*`, operate component by component. They accept
equal matrix dimensions or scalar broadcasting. `mul` performs algebraic
multiplication: `mul(floatRxK, floatKxC)` returns `floatRxC`; matrix/vector and
vector/matrix forms contract the matching dimension. A vector on the left is a
row vector; a vector on the right is a column vector. `transpose` swaps the
dimensions and `determinant` requires a square floating-point matrix.

One-dimensional matrix aliases, generic `matrix<T,R,C>` syntax, matrix member
swizzles, and explicit matrix storage-layout options remain unimplemented.
Row/column vector algebra does not itself define buffer storage layout.

## Resource operations

`ConstantBuffer<T>` currently requires a struct element. Its fields can contain
the supported scalar, vector, matrix, array, and nested struct types. Access
uses the usual field syntax, such as `parameters.scale`. Fields remain read-only,
including nested array elements, swizzles, and `out` arguments. Imported constant
buffers preserve their struct's field visibility. HLSL uses its native
`ConstantBuffer<T>` template; reflection records read access.

Direct `cbuffer` syntax, nonstruct constant-buffer elements, automatic conversion
of a constant-buffer handle to its element, buffer arrays, register annotations,
resource-valued fields, and target-specific byte layout remain unimplemented.

Byte-address buffers support `Load`, `Load2`, `Load3`, and `Load4`, returning
one to four `uint` components. Writable buffers provide matching `Store` methods.
`GetDimensions` returns the byte count through a writable `uint` argument.
`InterlockedAdd`, `InterlockedAnd`, `InterlockedOr`, `InterlockedXor`,
`InterlockedExchange`, and `InterlockedCompareExchange` support writable buffers
with an explicit `uint` original-value output. Addresses are byte offsets;
these signatures follow native HLSL methods.

Typed generic loads/stores, status outputs, optional atomic outputs, min/max and
64-bit atomics, and atomics on other resource families remain unimplemented.
The frontend does not prove byte-offset alignment, resource size, memory ordering,
or target capabilities.

Read-only textures support indexed loads at mip zero, except cube textures.
Read-write textures support indexed reads and writes. Coordinates have one, two,
or three integer components matching the texture dimension. Cube textures
support sampling with a three-component direction.

| Method | Implemented receiver and arguments |
| --- | --- |
| `Load` | Structured buffer: scalar index; read texture: integer coordinates plus mip; RW texture: integer coordinates |
| `Sample` | Floating-point read texture: `SamplerState`, floating-point coordinates |
| `SampleLevel` | Floating-point read texture: sampler, coordinates, explicit float LOD |
| `SampleCmp`, `SampleCmpLevelZero` | Floating-point 1D/2D/cube read texture: `SamplerComparisonState`, coordinates, float comparison value; returns float |
| `GetDimensions` | Structured buffer: uint count and stride outputs; texture: uint/float dimensions; read texture also accepts uint mip input and a level-count output |

Dimension outputs must be writable and use matching types. `Load` is unavailable
on cube textures. Resource methods lower to typed method calls in IR and native
HLSL calls. Texture arrays, multisampling, gather/gradient/bias/offset/status
overloads, other atomic overloads, resource binding declarations, and
target capability checks remain unimplemented. Implicit-LOD sampling must be
validated for the chosen target stage/profile; this frontend does not yet
perform a complete stage capability analysis.

## Intrinsics

The current built-in signatures cover `abs`, `sin`, `cos`, `tan`, `sqrt`, `rsqrt`,
`floor`, `ceil`, `frac`, `exp`, `exp2`, `log`, `log2`, `saturate`, `min`, `max`,
`pow`, `clamp`, `lerp`, `dot`, `length`, `normalize`, `mul`, `transpose`,
`determinant`, `all`, `any`,
`GroupMemoryBarrierWithGroupSync`, `DeviceMemoryBarrierWithGroupSync`, and
`AllMemoryBarrierWithGroupSync`.

These are limited signature rules rather than a port of the complete core
module. They do not provide full intrinsic overload compatibility or capability
validation.

## Compute entry points

```slang
RWStructuredBuffer<float> result;

[shader("compute")]
[numthreads(64, 1, 1)]
void main(uint3 id : SV_DispatchThreadID)
{
    result[id.x] = float(id.x);
}
```

`SV_GroupID` and `SV_GroupThreadID` require `uint3`; `SV_GroupIndex` requires
`uint`. Validation requires a `void` return, a function body, supported parameter
semantics, and a valid thread-group size.

## Vertex and fragment entry points

`[shader("vertex")]` and `[shader("fragment")]` identify graphics entry points;
`"pixel"` is an alias for `"fragment"`. They require a function body and reject
`numthreads`. Inputs and outputs can be scalar/vector parameters, `out`/`inout`
parameters, return values, or nested structures with semantics on their leaves.
Semantics are case-insensitive, with an omitted numeric suffix meaning zero.
Duplicate semantics within the inputs or within the outputs are rejected.

| Semantic | Implemented stage and direction | Type |
| --- | --- | --- |
| `SV_Position` | Vertex output, fragment input | `float4` |
| `SV_VertexID`, `SV_InstanceID` | Vertex input | `uint` |
| `SV_IsFrontFace` | Fragment input | `bool` |
| `SV_PrimitiveID`, `SV_SampleIndex` | Fragment input | `uint` |
| `SV_Coverage` | Fragment input or output | `uint` |
| `SV_Target0` through `SV_Target7` | Fragment output | Numeric scalar/vector |
| `SV_Depth`, `SV_DepthGreaterEqual`, `SV_DepthLessEqual` | Fragment output | `float` |
| User semantics such as `POSITION`, `TEXCOORD0` | Vertex input/output, fragment input | Numeric scalar/vector |

Vertex outputs require `SV_Position`. Integer fragment varyings require
`nointerpolation` on the parameter or field. These interface rules describe the
implemented HLSL subset and can be stricter than upstream's target-independent
frontend. Matrices/arrays in shader interfaces, whole-structure semantics, other
interpolation modes, stage capability analysis, and other shader stages remain
unimplemented. See `examples/vertex-transform.slang` and
`examples/fragment-color.slang` for a matching pair.

## Reflection

JSON reflection reports entry points, stages, thread-group sizes, global names
and type names, and read/read_write access for constant, structured, byte-address
buffers, and textures. It does not
provide binding indices, register spaces, byte offsets, strides, or layout.
Applications must not treat this JSON as the upstream compiler's ABI reflection.
Schema version 4 adds a `modules` graph and the optional owning `module` name
for imported globals. Each module lists its primary source and explicit member
source IDs, imports, and legacy visibility mode. Imported global names retain
their source spelling; `target_name` identifies their generated HLSL name.
Imported struct type names currently use canonical linked names.

Schema version 3 introduced `target_name` for entries and globals. Source identifiers
that conflict with HLSL keywords are renamed consistently; the source `name`
remains available. Pass the entry's `target_name` to DXC. As in schema version 2,
`thread_group_size` is omitted for graphics entries; compute entries retain
their three-element array.

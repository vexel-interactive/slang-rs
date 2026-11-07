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
- Matrix aliases such as `float2x3`, with 2-4 rows and 2-4 columns.
- One-dimensional arrays with a positive decimal literal size.
- `StructuredBuffer<T>` and `RWStructuredBuffer<T>` with indexed element access.
- `Texture1D/2D/3D/Cube<T>`, `RWTexture1D/2D/3D<T>`, `SamplerState`, and
  `SamplerComparisonState`. Texture elements are half/float/int/uint scalar or
  vector types; omitting `T` selects `float4`.

Constant buffers, pointers, 64-bit integers,
user-defined generics, and struct constructors are not implemented.

Supported modifiers are `const/static/uniform/groupshared` for globals,
`const/static` for locals, and `const/in/out/inout/nointerpolation` for parameters.
Structure fields support semantics and `nointerpolation`. Conflicting
parameter directions and `const out/inout` are rejected. Global values without
`static/groupshared` are read-only; RW buffer elements remain writable.
Global const declarations with initializers require `static`.
Complete modifier rules are not yet implemented. Comma-separated declarations,
default arguments, and aggregate initialization are not supported.

## Expressions and statements

Expressions include numeric and boolean literals, names, scalar, vector, and
matrix constructors, unary and binary operators, assignment, compound assignment,
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
overloads, byte-address buffers, atomics, resource binding declarations, and
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
and type names, and read/read_write access for structured buffers and textures. It does not
provide binding indices, register spaces, byte offsets, strides, or layout.
Applications must not treat this JSON as the upstream compiler's ABI reflection.
Schema version 3 adds `target_name` for entries and globals. Source identifiers
that conflict with HLSL keywords are renamed consistently; the source `name`
remains available. Pass the entry's `target_name` to DXC. As in schema version 2,
`thread_group_size` is omitted for graphics entries; compute entries retain
their three-element array.

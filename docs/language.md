# Supported language subset

## Numeric type applications

`vector<T, N>` and `matrix<T, R, C>` resolve to the same canonical types as
`float3`, `float3x3`, and the other scalar-prefixed names. The component argument
can use a scalar alias, including an imported alias. Type applications work in
declarations, aliases, resource arguments, arrays, and constructor calls.

| Source type | Canonical type |
| --- | --- |
| `vector<float, 3>` | `float3` |
| `vector<float, 1>` | `float1` |
| `matrix<double, 2, 3>` | `double2x3` |
| `matrix<float, 1, 1>` | `float1x1` |
| `vector`, `vector<>` | `float4` |
| `vector<int>` | `int4` |
| `matrix`, `matrix<>` | `float4x4` |
| `matrix<float, 2>` | `float2x4` |

Omitted arguments use the upstream core defaults: `float` for the component
and four for each dimension. Supplied dimensions currently require integer
literals from one through four; decimal, hexadecimal, and binary spellings are
accepted. Named constant or expression dimensions, the matrix layout argument,
user-defined generic types, and generic functions remain
unimplemented. Constructors require explicit angle brackets, including `<>`, or
a transparent type alias; bare `vector(...)` and `matrix(...)` calls do not
specialize a numeric type. They use the existing numeric and aggregate checks. See
[numeric-type-templates.slang](../examples/numeric-type-templates.slang).

One-component vectors remain distinct from scalars in arithmetic, reflection,
and overload selection. `float1` supports `.x`, `.r`, and `[0]`; scalar casts and
constructors extract its component. A matrix row is always a vector, including
the `float1` row of `float3x1`. Matrix values retain both dimensions: a `float1x1`
does not implicitly convert to a scalar or from a vector. One-component vectors
can be used as conditions. See
[one-component-shapes.slang](../examples/one-component-shapes.slang).

The source language uses Slang syntax. Version 0.1.0 implements a small subset
of upstream features. This document describes its actual boundaries; upstream's
language reference remains the specification for the complete port.

## Matrix component selectors

Matrices support `_m00` through `_m33` with zero-based coordinates, and `_11`
through `_44` with one-based coordinates, subject to their actual dimensions.
For example, `float2x3` allows `_m12` and `_23` for its last component. Combining
one to four selectors produces a scalar or vector, such as `_m00_m12` or
`_11_23`. One selection must use a single indexing convention.

Repeated components are readable. Assignment, compound assignment, increment,
and output arguments require distinct components and writable underlying storage.
Selectors retain the original location for RW buffer elements, intrinsic outputs,
and scalar integer atomics on shared matrices. Const matrices, read-only resource
elements, and temporary values are not writable. Struct fields named `_m00`
continue to use ordinary member lookup. See
[matrix-swizzles.slang](../examples/matrix-swizzles.slang).

## Preprocessing

Object and function macros support fixed and variadic parameters, nested calls,
and aliases. Directives include `#define`, `#undef`, `#include`, `#pragma once`,
`#if`, `#ifdef`, `#ifndef`, `#elif`, `#else`, `#endif`, `#line`, `#error`, and `#warning`.
Conditions support `defined`, integer operators, and short-circuit `&&`/`||`.

Include guards, line continuations, `#` stringification, `##` token pasting,
and `__VA_ARGS__` are supported. Arguments for `#` and `##` are used before macro
expansion. Pasted tokens are lexically validated and rescanned. Unused arguments
are not expanded.

Quoted includes search the current file's directory before `-I` paths. Angle
includes search `-I` paths. `MemoryIncludes` resolves exact names from a map.
`__LINE__` and `__FILE__` are predefined location macros. They use the displayed
line and file of the initiating macro call, including nested calls, multiline
arguments, token-pasted names, and definitions originating in an included file.
File names produce quoted strings with escaped backslashes, quotation marks,
and ASCII control characters.
Both names participate in `defined`; `#undef` removes them. A user definition
replaces a builtin with warning `P010`, and removing that definition does not
restore the builtin. `compile_source` and `compile_file` also supply `__SLANG__`
as one unless the caller supplies a definition. Imported modules start with
fresh location macros and the same caller definitions.

`__VA_OPT__`, GNU comma elision, other predefined macros, full redefinition
rules, and macro expansion stacks remain to be ported.

### Source line directives

`#line N "name"` assigns a displayed line and optional filename to subsequent
source locations. Arguments expand macros. Omitting the filename retains the
current displayed name; an integer filename becomes its token spelling. Both
`#line default` and an empty `#line` restore the physical name and line count.
Inactive conditional branches do not apply directives.

```slang
#define FIRST_LINE 40
#line FIRST_LINE "generated.slang"
static const int line = __LINE__; // 40
#line 10
static const int next = __LINE__; // 10, still generated.slang
#line default
```

Mappings affect diagnostics, `__LINE__`, and `__FILE__`. Diagnostic excerpts still
come from the physical source text. Includes, imports, member files, source IDs,
and byte spans retain their physical identity and resolution paths. A continued
directive counts physical lines from its keyword: `#line 50` followed by a
backslash-newline and a filename makes the following source line 51.

Numeric conversion follows upstream's directive rules rather than ordinary
integer-expression parsing: decimal and lowercase `0x` are recognized, suffixes
are ignored, signed 64-bit overflow saturates, and the result wraps to 32 bits.
For example, `4294967295` becomes -1; uppercase `0X10` and `0b100` become zero.
Zero and negative displayed lines are valid. Negative `__LINE__` values work in
preprocessor conditions, but upstream rejects them as typed integer literals;
the native compiler also reports `T010` for that use. Literal `#line -1`, arithmetic
expressions, and trailing flags are rejected.

Quoted filenames decode C byte escapes and Unicode escapes. Decoded names must
form UTF-8 in the native source API; arbitrary upstream byte filenames remain
unsupported. See [line-directives.slang](../tests/conformance/positive/line-directives.slang).

Limits: 64 include levels, 128 macro nesting levels, and 100000 steps per
expansion. `#if` uses signed i64 wrapping arithmetic; complete upstream
signed/unsigned evaluation rules are not implemented.

## Declarations and types

- Functions, prototypes, overloads, and `in`, `out`, `inout` parameters.
- Global and local variables, and simple structures with fields.
- `bool`, `int`, `uint`, `half`, `float`, `double`, and vectors with 1-4 components.
- Scalar names `int32_t`, `uint32_t`, `float32_t`, and `float64_t` resolve to the
  corresponding native scalar types.
- Matrix aliases such as `float2x3`, with 1-4 rows and 1-4 columns.
- Fixed arrays with positive decimal literal dimensions, including nested arrays.
- Global `typedef` and nongeneric `typealias` declarations.
- `StructuredBuffer<T>` and `RWStructuredBuffer<T>` with indexed element access.
- `ConstantBuffer<T>` with a struct element and read-only field access.
- `ByteAddressBuffer` and `RWByteAddressBuffer` with explicit byte-address methods.
- `Texture1D/2D/3D/Cube<T>`, `RWTexture1D/2D/3D<T>`, `SamplerState`, and
  `SamplerComparisonState`. Texture elements are half/float/int/uint scalar or
  vector types; omitting `T` selects `float4`.

Pointers, 64-bit integers,
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

Local `static` variables have private storage for the shader invocation. Storage
starts at zero; an explicit initializer runs once, when execution first reaches
the declaration. Later calls reuse the value. Initialization can depend on
parameters and call other functions. Groups, arrays, structures, imported
functions, and `for` initializers follow the same rule. This storage is separate
for each declaration and is absent from declaration reflection. It does not
persist across shader dispatches. See
[local-static-storage.slang](../examples/local-static-storage.slang).
Complete modifier rules are not yet implemented.

Comma-separated variable declarations share their leading type and modifiers.
Each declarator has its own array suffix, semantic, and initializer:
`float a[2] = {1, 2}, b = a[1], c[3];`. Local initializers can refer to earlier
declarators in the same group. Fields and globals retain declaration order;
grouped `for` initializers keep every variable inside the loop scope. Parentheses
enclose a comma operator inside one initializer. Type alias groups and mixed
function/variable declaration groups are unsupported. See
[multiple-declarations.slang](../examples/multiple-declarations.slang).

## Function default arguments

Input parameters may declare trailing default arguments:

```slang
float read(float value, float scale = 2.0, float bias = 1.0)
{
    return value * scale + bias;
}

float result = read(3.0); // read(3.0, 2.0, 1.0)
```

Each omitted argument evaluates its default when called. Defaults are checked
in the declaring module, and caller-local names cannot capture their global
references. Scalar, numeric aggregate, struct, and fixed-array defaults use the
same contextual initialization rules as variables. `out/inout` defaults and
required parameters following a default are rejected.

Calls use defaults from the first declaration of a signature. Later definitions
cannot add or replace that declaration's defaults. Overload ranking uses supplied arguments and does not
prefer a function because it needs fewer defaults. Calls may therefore be
ambiguous between a one-parameter function and a two-parameter function with a
default. Parameter references in defaults are not implemented. Recursive or
excessively branching default expansion produces a bounded lowering diagnostic.
See [default-arguments.slang](../examples/default-arguments.slang).

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
symbolic sizes, array return types,
and array elements inside resource types are not implemented. Alias chains and
resolved array ranks are each limited to 32. Local/nested aliases, generic
aliases, qualified type references, pointer declarators, and C-style casts using
user alias names remain unported. See [the alias example](../examples/type-aliases.slang).

Fixed arrays also support constructors: `float[3](1.0, 2.0)`, `float[3]()`,
and `Row(1.0, 2.0, 3.0)` through a transparent alias. Supplied values follow
initializer-list rules, including flat multidimensional arguments and nested
list boundaries. Remaining elements use zero or struct field defaults. A single
argument of the exact array type copies the array without evaluating defaults.
Leading dimensions wrap in source order: `float[3][2](...)` constructs two rows
of three floats. Constructor dimensions currently require positive decimal
literals and remain bounded to rank 32. See
[array-construction.slang](../examples/array-construction.slang).

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
owning module and evaluated outside the caller's local bindings. Supplied
constructor arguments use the caller's scope. Nested struct and array defaults
retain this distinction; see
[field-default-scope.slang](../examples/field-default-scope.slang).
Transparent aliases use the same constructor. Public structs
accept constructor arguments only for explicitly public fields, including when
called from their own module. Internal fields still receive their defaults.

Declarations without an initializer remain uninitialized and do not invoke
field defaults. User-defined constructors and resource-valued memberwise
construction remain unimplemented. Default
aggregate expansion is limited to 32 levels and 65536 elements. Recursive
constructor defaults report a diagnostic during lowering. See
[struct-construction.slang](../examples/struct-construction.slang).
Struct value graphs reject cycles and nesting beyond 128 levels independently
of initialization; repeated uses of a shared child type do not expand the graph.

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
`if/else`, `while`, `for`, `do/while`, `switch`, `break`, `continue`, and `discard`.
C-style numeric casts accept scalar names, scalar-prefixed vector/matrix names,
and explicit `vector<...>` or `matrix<...>` applications. Scalars can convert to
another scalar or splat into a vector or matrix. Vector casts preserve the width.
Matrix casts preserve or reduce both dimensions, selecting the top-left region.
Component conversions include boolean and numeric types. Vector truncation,
vector-to-scalar conversion, matrix expansion, and reshaping are rejected.
The operand is evaluated once. Standalone alias names and aggregate/resource
cast targets remain unsupported. See [numeric-casts.slang](../examples/numeric-casts.slang).
Statement attributes remain to be ported.

The comma operator evaluates its left operand before its right operand and
returns the right operand's value. It has lower precedence than assignment and
works in `for` initializers/increments, returns, and parenthesized expressions.
Parentheses keep a comma expression together when passed as one function
argument. Unparenthesized commas continue to separate arguments and initializers.
Comma results are not writable locations or supported integer constant
expressions. A brace list inside a comma expression cannot infer its type from
the surrounding declaration. The frontend does not reproduce upstream's comma
warnings. See [comma-expressions.slang](../examples/comma-expressions.slang).

The checker resolves nested scopes, validates conversions and scalar conditions,
and tracks writable locations. It rejects writes to const values, read-only
buffers, and swizzles with repeated components. Overload conversions currently
have equal cost after exact matches. Full overload ranking, definite assignment,
general constant evaluation, and uniformity analysis are absent. Stage requirement
checking covers the fragment and compute operations described below.

A local initializer resolves names before introducing that local. For example,
`float value = value + 1;` inside a nested scope reads an existing outer `value`.
Subsequent expressions use the new binding. Canonical local names preserve this
distinction when emitting HLSL, including parameter and const shadowing. See
[lexical-bindings.slang](../examples/lexical-bindings.slang).

Unbraced declarations inside `if`, `else`, `while`, and `do` belong to their
surrounding lexical scope. An `else` branch and later statements can use a
binding declared by the preceding branch. Explicit braces create a scope, and
`for` keeps its initializer and unbraced body declarations within its own scope.
Duplicate declarations in a scope are rejected. HLSL lowering separates storage
from conditional or repeated initialization to preserve these rules when adding
control-body braces. Literal const initializers retain their constant value.
Definite-assignment warnings remain unimplemented, so programs must initialize
values on every execution path before reading them. See
[unbraced-declarations.slang](../examples/unbraced-declarations.slang).

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
Scalar integer and boolean global constants resolve forward references through
their checked dependencies, including imported declarations. Cyclic integer
initializers are rejected even when the reference occurs in an inactive branch.
Checked case labels are emitted as integer literals after selector conversion,
including labels derived from local static const bindings.

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
`ConstantBuffer<T>` template; reflection records read access and the legacy HLSL
byte layout of the element. See [resources.md](resources.md#constant-buffer-byte-layout).

Direct `cbuffer Name : register(b2, space1) { float scale; };` declarations
inject read-only field names into global scope. The buffer name has its own
namespace and cannot be used as a value or to qualify its fields. The trailing
semicolon is optional. Nonstatic initializers are checked but do not initialize
buffer data; `const` fields still contain runtime data. Static fields become
private global storage and do not occupy buffer bytes. In declared modules,
exporting a field requires both a public buffer and a public field.
Reflection exposes one resource under the source buffer name and its element
layout; injected fields and static members are omitted from the global list.
Empty constant buffers retain their declarations and zero-byte layouts without
consuming a register. Duplicate buffer names within a module are rejected.
Direct fields support `packoffset(c2.y)` with explicit fields placed before
automatic ones; see [resources.md](resources.md#constant-buffer-byte-layout).
Ordinary struct fields also support packing, including nested structs and
arrays of values. Packed constant-buffer handle arrays remain unsupported.

Nonstruct constant-buffer elements, automatic conversion
of a constant-buffer handle to its element,
resource-valued fields, and byte layout for other targets remain unimplemented.

Fixed arrays of the supported resource handles, including constant buffers,
preserve their outer-to-inner dimensions. Indexing selects a resource handle;
its usual resource operations then apply. Read-only resource contents remain
read-only through array indexing. Structured buffer element arrays and
resource-valued buffer elements remain unsupported.

Global resources accept `: register(t0, space2)` and the corresponding `b`, `u`,
and `s` classes. Omitted spaces normalize to zero. Aliases, declaration groups,
imports, and macros preserve each resource's explicit binding. Resource arrays
occupy a contiguous range with one slot per element. Overlaps produce warning
`T023`; class mismatches and unsupported binding contexts produce error `T022`.
See [resources.md](resources.md) for syntax, reflected ranges, and current limits.
Resources without an annotation receive the first contiguous free range in
space zero, after reserving all explicit ranges and an implicit constant-buffer
slot for ordinary global uniforms. Unused resources still occupy their ranges.

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
| `SampleBias` | Floating-point read texture: sampler, coordinates, float LOD bias |
| `SampleGrad` | Floating-point read texture: sampler, coordinates, two gradients of the coordinate type |
| `SampleCmp`, `SampleCmpLevelZero` | Floating-point 1D/2D/cube read texture: `SamplerComparisonState`, coordinates, float comparison value; returns float |
| `GetDimensions` | Structured buffer: uint count and stride outputs; texture: uint/float dimensions; read texture also accepts uint mip input and a level-count output |

Dimension outputs must be writable and use matching types. `Load` is unavailable
on cube textures. `SampleGrad` supports 1D/2D/3D/cube textures; gradients use
float, float2, or float3 to match their coordinates. Explicit gradients work in
compute shaders without implicit derivative instructions. Resource methods
lower to typed method calls in IR and native HLSL calls. Texture arrays,
multisampling, gather/offset/status
overloads, other atomic overloads, resource binding declarations, and
target capability checks remain unimplemented. Implicit-LOD sampling must be
validated for the chosen target stage/profile; this frontend does not yet
perform a complete stage capability analysis.
`SampleBias`, like implicit-LOD sampling, requires a compatible target profile
when used outside fragment shaders. See
[texture-gradients.slang](../examples/texture-gradients.slang).

## Intrinsics

The current built-in signatures cover `abs`, `sin`, `cos`, `tan`, `sqrt`, `rsqrt`,
`floor`, `ceil`, `frac`, `exp`, `exp2`, `log`, `log2`, `saturate`, `min`, `max`,
`pow`, `clamp`, `lerp`, `dot`, `length`, `normalize`, `cross`, `distance`,
`reflect`, `refract`, `faceforward`, `mul`, `transpose`,
`determinant`, `all`, `any`, `clip`, `sincos`, `modf`, `frexp`,
`GroupMemoryBarrier`, `DeviceMemoryBarrier`, `AllMemoryBarrier`, and their
`WithGroupSync` variants.

Screen derivative signatures include `ddx`, `ddy`, `fwidth`, and their `_coarse`
and `_fine` variants. See the derivative and profile rules below.

These are limited signature rules rather than a port of the complete core
module. They do not provide full intrinsic overload compatibility or capability
validation.

`sincos(x, out sine, out cosine)` returns void. `modf(x, out integral)` returns
the fractional part. `frexp(x, out exponent)` returns the mantissa and writes an
`int` exponent with the same shape. Scalar, vector, and matrix forms support
half, float, and double; integer inputs promote to float. Matrix output functions
lower to row operations with reusable helpers; supplied matrix expressions
evaluate once. See
[matrix-output-intrinsics.slang](../examples/matrix-output-intrinsics.slang).

Output arguments participate in numeric type and shape inference. Selected input
conversions become explicit in IR, including scalar splats inferred from vector
outputs. Outputs must have exactly the selected type and be writable locations;
const values, temporary expressions, and repeated swizzles are rejected. `sincos`
and `modf` can infer brace-list input types from their outputs. See
[output-intrinsics.slang](../examples/output-intrinsics.slang).

## Geometric functions

`length`, `distance`, `normalize`, `reflect`, and `refract` accept scalars and
vectors with one through four components. Integer inputs promote to float;
mixed numeric components select half, float, or double from the supplied types.
`length` and `distance` return a scalar. `normalize`, `reflect`, and `refract`
retain the selected shape. `faceforward` returns a vector, including a
one-component vector for scalar inputs. Matrices and boolean inputs are rejected
by these functions.

Vector operands must have matching dimensions; ordinary scalars can splat to
the vector dimension. A one-component vector does not splat to a different
dimension in these generic signatures. `refract` requires a scalar refraction
index, with a one-component vector convertible to that scalar. A brace list
cannot supply the index. Other geometric operands can use contextual brace
lists when a supplied operand establishes a concrete shape.

`cross` returns a three-component vector and accepts scalar splats,
one-component numeric vectors, and three-component vectors. It retains integer
components for integer inputs. Boolean scalars and three-component boolean
vectors select an integer overload; one-component boolean vectors are rejected.
When bool participates, floating arguments convert to the selected integer type.
Native integer cross products lower to cached functions using integer component
multiplication and subtraction. Each supplied expression is evaluated once.

`dot` accepts numeric scalars and vectors with one through four components and
returns the selected scalar component type. Integer products retain int/uint;
bool selects an integer overload as for `cross`. Mixed numeric components and
scalar splats follow the same shape checks. Integer and double dot products use
cached component arithmetic helpers, preserving each input evaluation once and
avoiding HLSL float conversion or unsupported double dot instructions. Matrices
remain unsupported, and these calls are not evaluated by the integer constant
evaluator. See [dot-int.slang](../tests/conformance/positive/dot-int.slang).

The HLSL backend uses float instructions for the other double geometric calls
and converts results back to the checked double shape. This preserves the source signature,
with the precision limits of those HLSL operations. Full floating-point execution
equivalence and automatic differentiation remain unverified. See
[geometric-float.slang](../tests/conformance/positive/geometric-float.slang) and
[geometric-int.slang](../tests/conformance/positive/geometric-int.slang).

## Integer bit operations

`countbits`, `firstbithigh`, and `firstbitlow` return `uint` counts or positions
with the same scalar/vector width as their input. `reversebits` reverses all
32 bits and preserves the input's integer sign and width. Boolean scalar/vector
arguments promote to `int`. Floating inputs, matrices, and wider integer types
are outside these signatures.

The highest-bit search retains signed inputs: negative `int` values search for
the highest zero bit, while `uint` values search for the highest set bit. A search
without a qualifying bit returns `0xffffffffu`. These calls retain runtime
evaluation, including when used in function defaults. See
[integer-bit-operations.slang](../examples/integer-bit-operations.slang).

## Bit reinterpretation

`asint`, `asuint`, and `asfloat` reinterpret 32-bit numeric components while
preserving scalar, vector, or matrix shape. An input already having the result's
component type passes through unchanged. Boolean components first convert to
`int`, and half components first convert to `float`, matching the upstream
overload selection. These operations differ from numeric casts such as `float(bits)`.

`asuint(doubleValue, low, high)` splits a scalar double into its low and high
32-bit words. Both outputs must be writable scalar `uint` or `int` locations;
fields and RW buffer elements are supported. Signed outputs retain the same word
bits. Vector and matrix splitting overloads are absent from the upstream baseline.
`asdouble(low, high)` reconstructs a scalar double or `double2` from scalar or
two-component words, with explicit input conversions. One-component vectors
select the scalar overload. Wider vectors and matrices are rejected. A single
double argument to a 32-bit reinterpretation function is also rejected.
These intrinsics are not executed by the compile-time integer evaluator. See
[bitcasts.slang](../examples/bitcasts.slang).

## Integer atomic operations

Global `InterlockedAdd`, `InterlockedAnd`, `InterlockedOr`, `InterlockedXor`,
`InterlockedMin`, `InterlockedMax`, and `InterlockedExchange` accept an `int` or
`uint` destination and a value, with an optional output for the old value.
`InterlockedCompareExchange` requires destination, comparison, replacement, and
old-value output. `InterlockedCompareStore` takes the first three arguments.

Destinations must be writable `groupshared` storage or elements of RW structured
buffers or RW textures. Arrays, fields, and scalar components within this storage
keep their location. Locals, local static storage, ordinary static globals,
read-only resources, temporaries, and ordinary `inout` parameters are rejected.
The destination selects the scalar integer type. Input conversions are explicit
in IR; old-value outputs must match that type and be writable. These operations
do not imply a memory barrier. Floating-point and 64-bit atomics are unimplemented.
Byte-address buffer atomics retain their separate method signatures described
above. See [group-atomics.slang](../examples/group-atomics.slang).

## Fragment discard and stage requirements

`discard;` rejects the current fragment. `clip(value)` rejects it when any
component is negative; the supported signature accepts one numeric scalar,
vector, or matrix and returns void. Boolean values and initializer lists without
a concrete inferred type are rejected.

Compute and vertex entry points cannot reach either operation. The checker
propagates fragment requirements through the selected function overloads,
imports, used global initializers, and omitted function or struct field defaults.
Unused helpers and explicitly supplied arguments do not impose their defaults'
requirements. This analysis handles recursive call graphs without stack recursion.
Complete target, profile, and stage analysis remains unimplemented. Nonvoid
functions still require return paths even when
they contain a discard. See
[fragment-discard.slang](../examples/fragment-discard.slang).

## Shared memory and barriers

All six supported memory barriers take no arguments and return void.
`GroupMemoryBarrier` orders group memory, `DeviceMemoryBarrier` orders device
memory, and `AllMemoryBarrier` orders both. Their `WithGroupSync` variants also
synchronize threads in the group. Barriers do not make divergent group
synchronization safe; uniformity analysis remains unimplemented.

The native shader subset requires compute stage for reachable groupshared
storage, group/all memory barriers, and every `WithGroupSync` variant.
`DeviceMemoryBarrier` without synchronization is also allowed in graphics
shaders. These requirements propagate through selected overloads, used global
initializers, and omitted defaults. Local names that shadow shared globals and
explicitly supplied defaults do not inherit the hidden declaration's requirement.
Unused shared declarations do not restrict an entry point.

Slang 2026.21 accepts some graphics uses of shared memory and group barriers
while emitting HLSL that DXC rejects. The native frontend reports `T023` before
emission. It also requires compute for shared reads that an optimizing backend
could eliminate. Mesh, amplification, and node stages are not implemented. See
[memory-barriers.slang](../examples/memory-barriers.slang).

## Derivatives and shader profiles

`ddx` and `ddy` compute partial differences across screen-space quads. `fwidth`
returns the sum of the magnitudes of those differences. Coarse and fine variants
select the corresponding quad derivative instructions. Each takes one operand
and preserves its floating scalar, vector, or matrix shape. Integer operands
convert to the corresponding float shape. Boolean and aggregate operands are
rejected. HLSL double derivatives use float instructions; native lowering makes
the input narrowing and conversion back to the checked double shape explicit.

The functions are available in fragment and compute shaders. Vertex entries
cannot reach them. Compute entries need shader model 6.6 and a valid quad shape:
either `numthreads(x, 1, 1)` with x divisible by four, or x and y both even,
including groups with multiple z slices. Invalid shapes produce `T024`.
Requirements propagate through selected calls, globals, and omitted defaults.
Divergent derivative control flow remains outside the current uniformity analysis.

`-profile cs_6_0`, `vs_6_0`, or `ps_6_0` supplies a profile for the selected entry.
Without `-entry`, it applies to every entry, so their stages must match. The CLI
accepts shader model 6.0 through 6.9 profiles for compute, vertex, and fragment.
Absent a supplied profile, the compiler starts at 6.0 and chooses 6.6 for compute
derivatives. An explicitly lower compute profile is upgraded with warning `C003`.
Reflection schema version 5 introduced the resulting `target_profile`; use that
profile when compiling the HLSL. A profile selection does not yet validate the
complete target capability set. Unsupported stages and older models are rejected.

HLSL lacks coarse/fine `fwidth` builtins, so generated helpers evaluate their
input once and sum the corresponding absolute derivatives. See
[screen-derivatives.slang](../examples/screen-derivatives.slang) and
[compute-derivatives.slang](../examples/compute-derivatives.slang).

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

`numthreads` accepts scalar integer/boolean constants and integer expressions,
including shifts, conditional expressions, integer constructors, and direct
floating literals converted by truncation. Global constants may refer to later
declarations. Attributes resolve in global scope before parameters, so a parameter
with the same name does not hide a size constant. Dimensions must be positive;
x and y cannot exceed 1024, z cannot exceed 64, and the product cannot exceed 1024.
The computed dimensions also determine whether compute derivatives have valid
quad groups. Runtime values and ordinary function calls are rejected.

The constant cache currently covers scalar `int`, `uint`, and `bool`. General
floating constant arithmetic remains unsupported for attributes and switch labels;
fractional expressions retain their original values in emitted shader code.

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
interpolation modes, complete stage capability analysis, and other shader stages remain
unimplemented. See `examples/vertex-transform.slang` and
`examples/fragment-color.slang` for a matching pair.

## Reflection

JSON reflection reports entry points, stages, thread-group sizes, global names
and type names, and read/read_write access for constant, structured, byte-address
buffers, and textures, including arrays of those resource handles. Schema
version 7 reports a `binding` object for global shader resources, with register
`class`, `index`, `space`, `count`, and an `explicit` origin flag. Numeric uniforms
and private storage omit that field. Schema version 6 introduced explicit ranges;
version 7 adds automatic allocation and origin flags. Byte offsets, strides, and
complete target layout remain unimplemented.
Applications must not treat this JSON as the upstream compiler's ABI reflection.
Schema version 5 added `target_profile` for HLSL entries, including any required
shader model upgrade. Pass `target_name` and `target_profile` to DXC's `-E` and
`-T` options respectively.
Schema version 4 added a `modules` graph and the optional owning `module` name
for imported globals. Each module lists its primary source and explicit member
source IDs, imports, and legacy visibility mode. Imported global names retain
their source spelling; `target_name` identifies their generated HLSL name.
Imported struct type names currently use canonical linked names.

Schema version 3 introduced `target_name` for entries and globals. Source identifiers
that conflict with HLSL keywords are renamed consistently; the source `name`
remains available. Pass the entry's `target_name` to DXC. As in schema version 2,
`thread_group_size` is omitted for graphics entries; compute entries retain
their three-element array.

# Supported language subset

## Numeric literals

Integer tokens support decimal, hexadecimal (`0x`/`0X`), binary (`0b`/`0B`),
and octal (a leading zero). Octal tokens warn with `L005`; invalid base digits
report `L004`. Magnitudes fit in 64 bits. A single `l`/`L` denotes 32-bit long;
matching-case `ll`/`LL` starts type selection at 64 bits. One `u`/`U` marker
may precede or follow the width suffix. Mixed-case `lL` and repeated markers
are rejected. Pointer-sized `z`/`Z` accepts one unsigned marker in either order.

Unsigned decimal literals select `uint` when they fit, then `uint64_t`.
Signed decimal literals select `int`, then `int64_t`. Decimal values beyond
the signed 64-bit range select `uint64_t` and warn with `T030`. Nondecimal
values can select `uint` or `uint64_t` without that warning. Values exceeding
the unsigned 64-bit range report `T010`.

Direct prefix minus gives `-2147483648` type `int` and
`-9223372036854775808` type `int64_t`. Parentheses around the operand prevent
that exception: `-(2147483648)` retains `int64_t`. An explicit `LL` keeps
`-2147483648LL` at 64 bits. HLSL emission preserves these types and bit patterns,
including the two signed minima.

Decimal `z`/`Z` literals select `intptr_t` through the signed 64-bit maximum,
then `uintptr_t` with `T030`. Direct `-9223372036854775808Z` is `intptr_t`;
parenthesizing its magnitude retains `uintptr_t`. Nondecimal pointer literals
without `u`/`U` remain signed, including `0xffffffffffffffffZ`, which is -1
for the implemented HLSL target. An unsigned marker always selects `uintptr_t`.
Pointer and fixed-width integer types retain separate identities. See the
[pointer literal cases](../tests/conformance/positive/integer-pointer-literals.slang).

Floating literals support decimal fractions and `e` exponents, hexadecimal
fractions and `p` exponents, leading-dot forms, and the legacy `#INF` spelling.
The suffix selects the literal's type before overload resolution or conversion:

| Type | Suffixes |
| --- | --- |
| float | empty, `f`, `F` |
| half | `h`, `H`, `hf`, `HF`, `fh`, `FH` |
| double | `l`, `L`, `lf`, `LF`, `fl`, `FL` |

Mixed-case two-letter suffixes and floating suffixes on an integer token such
as `2f` are invalid. Decimal floats parse directly to float precision. Decimal
half values round from double to half with ties to even, matching upstream's
double-rounding rule. Hexadecimal significands truncate discarded bits and
warn with `T029`. Decimal overflow warns with `T028`; underflow to zero warns
with `T027`. These warnings preserve the converted zero or infinity. Explicit
`#INF` does not warn. HLSL uses valid target spellings, including `L` on finite
double literals and converted half values; half/float overloads receive distinct
names to preserve Slang's selected signatures.

Direct signed literals and conversions that preserve their floating type can
convert to integer dimensions. Rounding happens first: `int(1.99999999)` is
two, while `int(1.99999999lf)` is one. Parentheses around a floating operand
prevent literal-specific constant folding, so `- -2.5` is a valid dimension
but `-(-2.5)` is rejected. Named floating constants and floating arithmetic
remain outside the compile-time integer evaluator.

Preprocessor numbers use upstream's separate signed 32-bit conversion:
`010` means ten, lower-case `0x10` means sixteen, and `0X10` and `0b10` mean
zero. `#if` arithmetic wraps to 32 bits. Ordinary integer expressions retain
their source bases. See the
[literal fixtures](../tests/conformance/positive/literal-suffixes.slang).

## Character literals

A character literal has type `uint` and holds exactly one unescaped Unicode
code point or one escape value. For example, `'X'` is `88u`, `'\n'` is `10u`,
and `'\u{ffffffff}'` is `4294967295u`. Character escapes can hold any unsigned
32-bit value; they do not require a valid Unicode scalar.

Supported escapes include quotes, backslash, question mark, `\a`, `\b`, `\f`,
`\n`, `\r`, `\t`, and `\v`; one through three octal digits; hexadecimal
`\x...` and `\x{...}`; four-digit `\u...`, braced `\u{...}`, and eight-digit
`\U...`. Hexadecimal escape values must fit in 32 bits. Empty, multiple-value,
unknown, incomplete, and overflowing literals report `L006` at the token span.

Characters work in numeric expressions, overloads, constructors, constant
dimensions, switch labels, macros, imports, and declaration defaults. HLSL
receives the decoded unsigned integer. String value support remains limited
to existing preprocessor and attribute contexts. See the
[character values](../tests/conformance/positive/character-values.slang) and
[context checks](../tests/conformance/positive/character-contexts.slang).

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
and four for each dimension. Supplied dimensions evaluate to values from one
through four. Integer expressions support arithmetic, bit operations, supported
comparisons, conditional selection, integer casts, and named global/local
constants. Global dependencies can reference later declarations and are checked
in their declaration module. Type dimensions use 64-bit intermediate arithmetic,
matching upstream's compile-time shape folding. Boolean constants and numeric literals can convert
to integer dimensions; floating literals truncate toward zero. For example,
`static const int Count = 2; vector<float, Count + 1> value;` declares a `float3`.

As in upstream's template grammar, `>`, `>=`, and `>>` cannot appear within a
type argument, including parentheses. Compute those operations in a named
constant before using the result as a dimension. Cyclic or excessively nested
shape dependencies report `T026`; dimension expressions must be compile-time
constants. General floating constant evaluation, constant function execution,
the matrix layout argument and general generic inference
remain unimplemented. Constructors require explicit angle brackets, including `<>`, or
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

## Function inlining

`[ForceInline]` takes no arguments and expands functions with numeric, aggregate,
or resource parameters and results before scalar constant propagation.
Void functions also expand, including empty bodies and early returns. The body
may contain ordinary value declarations, nested lexical scopes, branches, loops,
switches, and multiple returns. Nested inline calls, prototypes, selected overloads, and imported
functions are supported. Each argument evaluates once in source order, including
unused inputs. Assignments to an input parameter affect its captured copy.
Conditional and short-circuit calls keep argument evaluation in the selected
path. Loop conditions and increments retain their evaluation points, including
`continue` and early return. Static locals retain their shared per-invocation
storage across inlined calls. Surrounding expressions capture earlier values
before later statement effects and retain writable indexes for assignments and
output arguments. See [force-inline-values.slang](../tests/conformance/positive/force-inline-values.slang)
and [force-inline-control.slang](../tests/conformance/positive/force-inline-control.slang).

Out/inout arguments retain their original address when possible, including scalar
vector/matrix selections. Resource receivers and indexes evaluate once at their
argument position. Multi-component swizzles and resource subscript setters use
temporary storage with ordered copy-back after the body and before its captured
result is consumed. A member or implicit integer view of a resource element can
request direct addressing. Pure `out` arguments do not read the original value.
Output types must match, or have the same numeric shape and integral component
width. Same-width integer views retain aliases and convert loads and stores to
the declared parameter type. Floating, width-changing and scalar/vector output
conversions report `T015`. Dynamic indexes into writable numeric swizzles are
rejected; direct vector, matrix, array and resource indexes remain supported.
An initializer that shadows an outer name evaluates using the outer binding.
See [output examples](../tests/conformance/positive/force-inline-outputs.slang)
and [16/64-bit views](../tests/conformance/positive/force-inline-output-views.slang).
The pinned HLSL backend erases integer address casts, producing arithmetic
right shifts for some inlined unsigned 16/32-bit views of signed storage. The
native explicit conversions preserve logical shifts; separate exact artifact
goldens record four differing words. Potentially aliased output arguments may
have undefined behavior according to upstream's warnings; the regression checks
compiled artifacts and does not establish a language guarantee for such calls.

Global calls expand inside generated initializer functions with ordinary local
storage. Fixed-array initializer results use return wrappers. File-scope
declarations never receive lifted statement expressions. Static const initializers
reject mutable global references and ordinary user function calls with `T031`;
numeric/aggregate constructors and intrinsics remain eligible when their arguments
are constant. This validates eligibility without evaluating arbitrary function
bodies as compile-time functions.

Inlining shared/volatile local storage is not implemented and reports
`I005`. Statement-valued vector conditional/logical operands also report `I005`.
Recursive or excessive
expansion is bounded by depth and node limits. Ordinary calls retain their target
call boundary; they are not evaluated as compile-time functions for type shapes.
Array and struct inputs retain value-copy semantics. Resource inputs and results
retain handle identity. See [aggregate/void examples](../tests/conformance/positive/force-inline-aggregates.slang)
and [resource examples](../tests/conformance/positive/force-inline-resources.slang).
Dynamic ConstantBuffer handle selection remains an HLSL lowering limitation:
both the native and pinned upstream artifacts in the dedicated regression are
rejected by DXC when the selected handle is loaded as a struct value.

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

## Matrix storage order

`row_major` and `column_major` qualify matrix globals, struct and constant-buffer
fields, parameters, locals, return types, aliases, and numeric casts. Arrays of
matrices preserve the element qualifier at every dimension. Applying either
modifier to a non-matrix type reports `T001`; repeated modifiers select the
last one, and a declaration qualifier overrides an inherited alias qualifier.
The modifier affects storage while preserving matrix arithmetic, dimensions,
indexing, and value overloads.

The compilation default is column-major. Select another default with
`-matrix-layout-row-major` or `CompileOptions.matrix_major`; explicit source
qualifiers still take precedence. Structured-buffer matrix elements can also
be qualified inside their type argument, such as
`RWStructuredBuffer<column_major float2x3>`. Resource handles with different
element storage orders remain distinct types. A qualifier matching the default
resolves to the ordinary resource type.

Matrix arrays loaded from buffers convert by logical element values when their
order differs from the default. Copies, nested arrays, function arguments, and
read/write structured-buffer locations are covered by DXC checks. Matrix layout
template arguments
and layouts for other targets remain unimplemented.

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
expansion. `#if` uses signed 32-bit wrapping arithmetic; complete upstream
signed/unsigned evaluation rules are not implemented.

## Declarations and types

- Functions, prototypes, overloads, and `in`, `out`, `inout` parameters.
- Global and local variables, and simple structures with fields.
- `bool`, `int16_t`, `uint16_t`, `int`, `uint`, `int64_t`, `uint64_t`, `intptr_t`, `uintptr_t`, `half`, `float`, and `double`.
- Core names `int32_t`, `uint32_t`, `float16_t`, `float32_t`, and `float64_t`
  resolve to the corresponding scalar types and their vector/matrix forms,
  such as `float32_t3` and `int32_t2x2`. `float16_t` aliases `half`.
- Matrix aliases such as `float2x3`, with 1-4 rows and 1-4 columns.
- Fixed arrays with nonnegative constant dimensions, including nested and empty arrays.
- Global `typedef` and `typealias` declarations, including typealias templates.
- `StructuredBuffer<T>` and `RWStructuredBuffer<T>` with indexed element access.
- `ConstantBuffer<T>` with scalar, vector, matrix, fixed-array, struct, or void elements and read-only value access.
- `ByteAddressBuffer` and `RWByteAddressBuffer` with explicit byte-address methods.
- `Texture1D/2D/3D/Cube<T>`, `RWTexture1D/2D/3D<T>`, `SamplerState`, and
  `SamplerComparisonState`. Texture elements are half/float/int/uint scalar or
  vector types; omitting `T` selects `float4`.

Pointer declarators remain unimplemented. Explicit user type templates are described below.

Wide integer aliases include `int64_t2` and `uint64_t2x3`; numeric templates,
constructors, casts, overloads, arrays, and structured buffers use the same
canonical types. HLSL uses `vector<int64_t, N>` and `matrix<uint64_t, R, C>`.
Fixed-width core aliases preserve canonical identity through constructors,
overloads, numeric templates and buffer elements. For example, `float16_t2`
and `half2` are the same type. Without native 16-bit storage, emitted HLSL
half components occupy four bytes; the alias does not enable a different
storage mode. `int16_t` and `uint16_t` have distinct types, core vector/matrix
aliases, and truncating conversions. Their declarations select native storage
and require shader model 6.2. `-enable-16bit-types` or a profile of at least 6.2
also selects two-byte half storage. See the
[core alias cases](../tests/conformance/positive/core-numeric-aliases.slang).
Narrow scalar shifts saturate counts at 16, producing zero or signed right-shift
extension. Constant propagation also saturates proven scalar local compound
shifts at their 16/32/64-bit operand width. Indexed compound locations and
dynamic scalar operands retain runtime DXC behavior. See the
[compound shift fixture](../tests/conformance/positive/integer16-compound-shifts.slang).
Overloaded functions involving wide integers receive distinct target names
to preserve Slang's literal and conversion ranking. Mixed 64-bit arithmetic
operands are explicitly converted to the checked common type.

Pointer integers support numeric templates, constructors, casts, arrays,
structured buffers, constants and switches. `ssize_t` aliases `intptr_t`;
`size_t` and `usize_t` alias `uintptr_t`. Pointer vectors and matrices use
`vector<intptr_t, N>` and `matrix<uintptr_t, R, C>` with dimensions 1-4;
core aliases such as `intptr_t2` are absent. HLSL maps pointer integers to
`int64_t`/`uint64_t` using upstream's eight-byte generic pointer size,
independently of the compiler host. Other target pointer widths remain unported.

Binary arithmetic selects the larger component size; floating types take
precedence over integers. For distinct equal-width integer types, a signed
left operand selects the right type, and an unsigned left operand selects its
own type. Thus `intptr_t + int64_t` yields `int64_t`, while reversing those
operands yields `intptr_t`. Conditional branches and generic math functions
instead join by conversion cost, so both signed pairs select `int64_t`.
Pointer conversion rank lies between 32 and 64 bits; unsigned-to-signed pointer
conversions do not receive a lossless-promotion exemption.

Direct integer literals that fit a parameter's bit size receive upstream's
value-aware conversion ranking. A literal and a variable of the same type can
therefore select different overloads. Exact types remain preferred; tied
conversions remain ambiguous. See the
[promotion cases](../tests/conformance/positive/integer-pointer-promotions.slang) and
[inference cases](../tests/conformance/positive/integer-pointer-intrinsics.slang).

Compound arithmetic and bitwise assignments select their operand type from
the writable left argument. The right argument converts to that type before
the operation: `uint value=9; value/=int64_t(-2);` gives zero, while
`int64_t value=-9; value/=uint(2);` gives negative four. Integer destinations
also convert floating right operands before arithmetic. Compound shifts retain
an independent integer count type and reject floating counts. Emitted compound
assignments evaluate indexed destinations once. Wide conditional branches and
`min`/`max`/`clamp`/`mul` inputs explicitly preserve their selected component type.

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

Array dimensions use the same scalar constant evaluation as numeric type
applications. Declarations and constructors accept forward global constants,
lexical local constants, arithmetic, bit operations, comparisons, booleans, and
floating literals. For example, `float values[Count + 1]` and
`float[6 >> 1](1, 2, 3)` resolve their dimensions before checking initialization.
Negative or runtime dimensions report `T002`; dependency cycles report `T026`.

Arrays can appear as variables, struct fields, parameters, and constant-buffer
elements and function return types. Unsized arrays remain unimplemented.
Alias chains and
resolved array ranks are each limited to 32. Local aliases,
pointer declarators, and C-style casts using
user alias names remain unported. See [the alias example](../examples/type-aliases.slang).

Fixed arrays also support constructors: `float[3](1.0, 2.0)`, `float[3]()`,
and `Row(1.0, 2.0, 3.0)` through a transparent alias. Supplied values follow
initializer-list rules, including flat multidimensional arguments and nested
list boundaries. Remaining elements use zero or struct field defaults. A single
argument of the exact array type copies the array without evaluating defaults.
Leading dimensions wrap in source order: `float[3][2](...)` constructs two rows
of three floats. Constructor dimensions use constant expressions and remain
bounded to rank 32. See
[array-construction.slang](../examples/array-construction.slang).

Functions can return fixed arrays through aliases or leading array suffixes.
Return expressions use the same exact-copy and initializer-list checks as
variables. Prototypes, overloads, defaults, imported functions, and arrays of
resource handles preserve their source types. HLSL wraps nonempty results in
private structures, exposing their array values at each call. Matrix return
qualifiers move to the wrapper field. Shader entry points cannot return arrays.
The native checker still reports `T005` when a non-void function can fall through;
upstream accepts that case with warning `E41010` and an undefined return value.

Zero dimensions are valid: `float values[0]` stores no elements, and
`float[0]()` constructs its empty value. Any zero dimension makes a nested array
empty. Empty arrays retain their element types and dimension order in reflection
and overload selection. Copies, empty-array returns, and struct defaults are
supported. They occupy no uniform bytes or resource registers, including empty
resource fields in structs. HLSL uses distinct empty structures and reconstructs
empty fields outside physical buffers to preserve neighboring byte offsets.
Constant indices into zero-length arrays report `T013`. Dynamic numeric indices
have undefined source values; target legalization evaluates their operands once
and uses private scratch locations. Dynamic indexing with resource-valued
elements reports `I006` until undefined resource handles can be legalized.

## Struct construction and field defaults

Without a user-defined `__init`, named constructors initialize data-only structs
in field declaration order:

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

Declarations of these structs without an initializer remain uninitialized and
do not invoke field defaults. Resource-valued memberwise construction remains
unimplemented. Default aggregate expansion is limited to 32 levels and 65536
elements. Recursive
constructor defaults report a diagnostic during lowering. See
[struct-construction.slang](../examples/struct-construction.slang).
Struct value graphs reject cycles and nesting beyond 128 levels independently
of initialization; repeated uses of a shared child type do not expand the graph.

## User-defined struct constructors

Structs can declare overloaded `__init` members with parameter defaults and
`out`/`inout` parameters. `[ForceInline]` expands the selected constructor body:

```slang
struct Item
{
    int value = 3;
    [ForceInline] __init() {}
    [ForceInline] __init(int x) { value = x + 2; }
};

Item a;             // invokes __init(), value = 3
const Item b;       // local const declarations also invoke __init()
Item c = Item(5);   // value = 7
Item d = {5};       // selects the same constructor
Item e = Item(c);   // exact copies preserve the source value
```

The local declarations above belong inside a function. Static const globals
require an explicit initializer. Declaring a nongeneric `__init` disables synthesized
memberwise calls; the implicit exact copy remains available even when an
explicit copy constructor is declared. Overload selection, parameter defaults,
output writability and imported access rules follow ordinary function checking.
Nongeneric constructor prototypes and definitions remain separate overload candidates:
repeating the same signature makes a call ambiguous, as in the pinned upstream.

Declared field defaults execute before the constructor body in the struct's
owning module. Nested struct fields with a default constructor invoke it.
Unqualified fields refer to the receiver before module globals; parameters and
locals take priority over fields. `this` names the writable receiver. A bare
`return;` or falling through returns that receiver; `return Item(...);` returns
the specified value. Fields without defaults or constructor writes remain
uninitialized. Array declarations and array fields without explicit initializers
do not automatically construct their elements. A struct declaration with no
matching default constructor also remains uninitialized.

See [constructor calls](../tests/conformance/positive/user-constructors.slang),
[default construction](../tests/conformance/positive/user-constructors-defaults.slang)
and [global initialization](../tests/conformance/positive/user-constructors-global.slang).
Unconstrained constructor type parameters are supported as described below.
Definite-assignment warnings remain unimplemented.

## Generic struct constructors

`__init<T>` infers its type parameters from value arguments. Constructor
parameters remain independent of enclosing type parameters:

```slang
struct Marker
{
    int value;
    __init<T>(T ignored) { value = 17; }
};
Marker a = Marker(3);     // synthesized memberwise constructor: value = 3
Marker b = Marker(3.0);   // exact generic match: value = 17

struct Box<T>
{
    T value;
    __init<U>(U ignored, T input) { value = input; }
};
Box<int> box = Box<int>(2.0, 3);
```

Generic constructors preserve synthesized memberwise candidates when the owner
has no nongeneric constructor. Conversion cost, default type argument count and
ordinary/generic preference determine the selected candidate. Exact explicit
copies preserve the source value. A contextual list such as `Marker b = {a}`
can select a generic constructor when its value cannot initialize a memberwise
field. Nested lists, fixed-array elements, returns and function defaults use the
same constructor selection and inference path.

Type defaults follow first-declaration prototype rules, including dependent
defaults. Generic constructor prototypes join their definitions. Original
unused bodies receive opaque checking; only selected concrete bodies and
defaults activate their dependencies. Constructors support ordinary output
parameters, private caller argument types, resource handles, generic/nested
owners, field scope, global initializers and `ForceInline`. Their type parameters
are inferred through the owner call; `Marker.__init<int>(...)` is not a member
call syntax. Bare declarations and constructor fields do not automatically
invoke a generic constructor, even when its parameters have defaults.

See [selection and outputs](../tests/conformance/positive/generic-constructor-values.slang),
[owner scopes and lists](../tests/conformance/positive/generic-constructor-owners.slang),
[resource values](../tests/conformance/positive/generic-constructor-resources.slang),
[inlining and globals](../tests/conformance/positive/generic-constructor-inline.slang)
and [imports](../tests/conformance/positive/generic-constructor-module/main.slang).
Constraints, value parameters, complete polymorphic candidate ranking and GPU
execution parity remain incomplete. Recursive constructor specialization has
frontend coverage without shader recursion validation.

## Struct methods

Structs and specialized type templates can declare instance and static methods. Instance methods
are nonmutating by default; `[mutating]` makes `this` writable and requires a
writable receiver at the call site. `[nonmutating]` explicitly selects the default.

```slang
struct Counter
{
    int value;
    int read() { return value; }
    int twice() { return read() + this.read(); }
    [mutating] void add(int delta) { value += delta; }
    static int offset(int x = 2) { return x + 7; }
};

// Inside a function:
Counter c = Counter(3);
c.add(2);                    // c.value = 5
int result = c.twice();       // 10
int offset = Counter.offset(); // 9
```

Parameters and locals shadow implicit members; members take priority over module
globals. Methods can call other methods without spelling `this`, including from
a constructor. Static methods cannot access instance fields or call instance
methods without an explicit receiver. Static calls may use a type alias or a
value as their base. As in upstream, `makeCounter().offset()` does not evaluate
`makeCounter()` when `offset` is static.

Nonmutating calls take a receiver value before explicit arguments evaluate.
They accept const locals, temporaries, read-only buffer elements and loaded
constant-buffer values. Mutating calls retain the receiver's writable storage,
including nested fields, indexed arrays and writable resource elements. Receiver
coordinates evaluate once before explicit arguments. `[ForceInline]` uses the
same capture/address rules as ordinary input and inout parameters.

Methods support selected overloads, prototype/definition merging, trailing input
defaults in their owning module, output arguments and fixed-array results.
Imported methods require `public`; internal methods remain callable inside their
module. Explicitly supplied arguments keep the caller's scope. Calling a method
in a static const global initializer is an ordinary user call and reports `T031`.

See [method calls and effects](../tests/conformance/positive/struct-methods.slang),
[resource receivers](../tests/conformance/positive/struct-methods-resources.slang)
and [imported methods](../tests/conformance/positive/struct-methods-module/main.slang).
Extensions, properties, class methods, `[constref]`/`[__ref]`
receiver modes and first-class method values remain unimplemented.
Receiver-dependent parameter defaults report `T022`; the pinned oracle also
fails to lower the probed field/method defaults, which is not a claim that their
source syntax is invalid. C++ trailing `const` requires the HLSL dialect upstream;
the native Slang parser does not implement that dialect extension.

## Struct static fields

Nongeneric structs can declare `static` fields containing scalars, vectors,
matrices, fixed arrays, structs or resource handles. Each declaration has one
shared storage location per shader invocation, independent of the number of
struct instances. Static fields do not occupy instance storage or participate
in memberwise construction.

```slang
struct Counter
{
    static const int Count = 3;
    static int value = 4;
    int items[Count];
    void add() { value++; }
};
```

Type names, aliases, `this`, ordinary values and resource element values can
select a static field using `.` or `::`. Scope lookup with `::` permits only
static members even when the base is a value. The lookup base is checked but never evaluated:
`makeCounter().value` neither calls `makeCounter` nor loads a resource value.
Nonmutating methods and const receivers can write mutable static fields.
`static const` fields reject writes and require an initializer. Uninitialized
mutable static fields remain undefined; definite-assignment warnings are absent.

Static names shadow module globals in field defaults, method signatures and
bodies, and static field initializers. Parameters and locals retain priority.
Forward integer/boolean constants work in array and vector/matrix dimensions
and thread-group attributes. A default such as `int x = this.Count` uses only
static storage and does not require a receiver-dependent default.

Ordinary function and static method calls are allowed in a static const **member**
initializer. They do not become compile-time integer values unless the existing
constant evaluator can fold them. The global-only runtime initializer restriction
does not apply to these member declarations; ordinary user calls in a static const
global still report `T031`.

Imported static fields require `public`, and public fields require a public owner
and public value type. Reflection exposes their qualified source names, such as
`Counter.value`, separately from target names and module provenance.

General upstream global constant graph semantics remain unported. The
[shared-state fixture](../tests/conformance/positive/struct-static-fields.slang)
records one difference: a global static const initialized from mutable member
storage keeps its initialization value in native HLSL, while Slang 2026.21's
constant IR operand aliases the mutable storage. Static resource aliases pass
native DXC checks, but the pinned oracle fails HLSL emission for the
[resource-alias fixture](../tests/native/static-resource-members.slang).
Properties and other target layouts remain absent.

## Nested structs and aliases

Structs can contain structs, `typedef` and `typealias` members.
Select them with `.` or `::` in declarations, function signatures, resource
arguments and constructors:

```slang
struct Outer
{
    typedef int Number;
    struct Inner
    {
        Number value;
        Number read() { return value; }
    };
};
Outer.Inner value = Outer::Inner(3);
```

Nested structs retain distinct identities across owners and modules. Aliases
remain transparent, including array, resource and matrix-layout aliases.
Members may refer to later sibling types, and nested bodies can use enclosing
types and static members. The nearest member declaration wins over enclosing
and module names; parameters and locals retain priority. A nested instance has
no implicit receiver for instance fields or methods of its enclosing struct.
Explicit `Outer.Inner.member` lookup searches only `Inner`.

A struct-valued variable can also qualify a type, as in `owner.Inner value`.
Constructor selection through `makeOwner().Inner(3)` checks the base but never
evaluates it. Its side effects, initialization dependencies, parameter references
and shader-stage requirements do not enter the constructor call.

Imported nested types require public access at every selected member, and the
owner must be accessible. Their short names are not imported into global scope.
Legacy visibility also applies to nested declarations. Reflection preserves
qualified static member names such as `Outer.Inner.count`; canonical value types
and HLSL use separate generated names. Struct declaration depth is limited to
32, while alias and value-type cycles receive bounded diagnostics.
Local type declarations, namespace declarations,
and general expression syntax in type positions remain unported. See the
[nested-type fixture](../tests/conformance/positive/nested-types.slang).

## User type templates

Structs and `typealias` declarations support type parameters and defaults:

```slang
struct Pair<T = int, U = T> { T first; U second; };
typealias IntegerPair<T = int> = Pair<T, int>;
Pair<> first = Pair<>(2, 3);
IntegerPair<float> second = {4, 5};
```

Each application supplies explicit angle brackets, including `<>` when defaults
supply every argument. Defaults can refer to preceding parameters. Transparent
aliases, scalar spellings and supported computed array/vector/matrix dimensions
share a canonical specialization. Distinct argument types retain distinct fields,
constructors, methods and static storage. Generic owners can contain ordinary
nested types or further templates, such as `Outer<int>.Item<float>`; nested
defaults retain enclosing arguments. Imported templates accept caller-defined
type arguments while preserving their declaration module's value visibility.

Expansion is bounded to 1,024 concrete applications, 32 nested argument expansions
and 65,536 cloned syntax nodes. Diagnostics retain the template's source location.
Recursive value types still use the existing bounded value-type checker, even
when upstream would discard an unused recursive value. Canonical shape folding
uses the currently supported scalar constant evaluator, including lexical locals;
general constant function execution remains absent.

Before concrete specialization, templates are checked with distinct opaque
parameters through the ordinary semantic checker. Unused bodies receive name,
type, initialization, call, scope and control-flow checks. A parameter supports
copying and default construction with `T(value)` and `T()`; unrelated numeric
conversion and arithmetic require unsupported conversion/operator constraints.
Member calls can use explicit interface constraints as described below.
Unconstrained `T()` recursively zeroes numeric and aggregate values, without field
defaults or user `__init` calls. Exact `T(value)` preserves its input, including
constant-buffer handles. Ordinary concrete struct construction still applies
field defaults and selected non-copy constructors. Resource default construction
remains unsupported.
Parameter casts `(T)value` preserve exact scalar, vector, matrix, struct, fixed-array
and resource values, including captured parameters in nested templates. Copy casts
of `vector<T, N>` and `matrix<T, R, C>` retain their component identities. A cast
such as `(T)3` still requires a conversion constraint and fails even in an unused
template. Parenthesized value expressions keep their ordinary grouping meaning.
Symbolic vectors/matrices support copies, indexing, component selection and matrix
layout modifiers while retaining their opaque component types. Symbolic types
are discarded before concrete lowering and never appear in emitted artifacts.

This does not implement general constrained polymorphism. General witnesses,
value parameters and partial type-template applications remain unported.
The existing restrictions on local names matching module type aliases and on
fallthrough from nonvoid functions also apply inside checked templates. Full
polymorphic conversions, overload ranking and receiver-dependent defaults remain
incomplete.

The [type fixture](../tests/conformance/positive/generic-types.slang) records one
oracle optimization difference: Slang 2026.21 reuses a repeated static method call
despite its write to mutable member storage. Native HLSL retains both calls, and
separate exact DXIL goldens record the differing counter results.

## Generic functions

Free functions accept type parameters, defaults, explicit applications and inference:

```slang
T copy<T>(T value) { return (T)value; }
T pick<T, U = T>(T first, U second = U()) { return first; }
int value = copy<int>(3);
int selected = pick<int>(value);
int inferred = copy(value);
float partial = pick<int>(value, 2.0);
int empty = copy<>(value);
```

Function templates use the same canonical type arguments, bounded expansion and
opaque body checking as user type templates. Unused bodies are checked and unused
valid functions create no concrete IR. Calls between templates, prototype/definition
pairs, independently specialized overloads, declared value defaults,
`out`/`inout` arguments, aggregate/array/resource values and imported caller-defined
types use ordinary concrete checking and lowering. Parameter names on prototypes
and definitions can differ. Explicit template calls remain distinct from ordinary
overloads and local value comparisons.

Each overload retains its own type parameter count and defaults. Prototypes and
definitions share a signature after scalar/array shape and transparent alias
normalization. Their type defaults come from the first declaration; later defaults
are checked but do not replace or supplement that list. Explicit applications
consider overloads whose type arguments are supplied or defaulted. Concrete value
conversion costs select a candidate, with fewer default type arguments breaking
equal-cost ties. Distinct generic declarations can produce equal concrete input
types; such calls receive an ambiguity diagnostic instead of a declaration error.
Duplicate original signatures are rejected even when unused. Imported candidates
retain their individual visibility and caller type-argument contexts.

Omitted function type arguments are inferred from checked value argument types
before applying defaults. This includes partial explicit applications and `<>`.
Repeated parameters join compatible numeric types, including scalar splats into
vectors/matrices; identical aggregates and resource handles retain their type.
Different fixed-array, struct or resource types cannot be numerically joined.
Structural patterns infer vector/matrix components, array elements, resource
elements and user template arguments, including enclosing owners of nested types.
Aliases are transparent. A bare `T` preserves a constant-buffer handle, while
numeric patterns can infer from its implicitly loaded element.

Nested calls and calls inside unused templates use the ordinary semantic
expression types. Inference runs to a bounded fixed point, followed by a fresh
complete check before lowering. A call with no value arguments can use its default
type chain, as in `zero()`; an undetermined parameter without a default is an error.
Bare calls compare ordinary and generic candidates together. Ordinary overloads
win equal conversion/default-cost ties. Explicit angle brackets select generic
candidates. Output arguments still require valid writable locations.

Initializer lists do not infer a bare `T`, even when another argument supplies a
concrete type. Explicit `T` arguments can provide their context. A structural
numeric pattern such as `vector<T, 2>` can use an inferred component from another
argument to initialize a list. Candidate signatures reject invalid compound
type substitutions before creating function instances.

Concrete candidate bodies and parameter defaults are expanded and checked only
after selection. Generated owner methods follow the same rule. Type declarations,
field initializers and static storage needed only by unselected signatures are
removed before the final check, emission and reflection. Selected bodies, defaults
and constructor initialization plans activate their dependencies until inference
and activation reach a fixed point. Original templates still receive opaque body
checking even when none of their concrete instances is selected. Ambiguous calls
remain errors before dormant candidates are removed.

The [deferred body fixture](../tests/conformance/positive/generic-deferred-bodies.slang)
checks ordinary overloads winning over resource-valued generic candidates whose
bodies or defaults use unsupported concrete construction. The
[signature fixture](../tests/conformance/positive/generic-deferred-signatures.slang)
checks that an unselected aggregate dependency contributes no static resource.
The [constructor fixture](../tests/conformance/positive/generic-deferred-constructors.slang)
checks automatic, brace, explicit, nested and default-argument construction on
specialized owners, including mutable static state.

Value parameters, general constraints, higher-order function
values and entry-point specialization remain unported. Complete constrained
overload specialization and ranking remain incomplete. Selected resource `T()`
construction and unused resource-component vector locals still encounter native
implementation limits that upstream accepts.

## Generic struct methods

Instance and static methods support unconstrained type parameters independently
of their enclosing owners:

```slang
struct Box<T>
{
    T value;
    U copy<U>(U input) { return input; }
    T read() { return copy(value); }
    static U zero<U = int>(U input = U()) { return input; }
};
Box<int> box = {2};
float value = box.copy<float>(3.0);
int empty = Box<int>.zero();
```

Explicit, partial and omitted method arguments use the function inference rules.
The receiver determines its owner; only the method's own input parameter patterns
infer its independent type parameters. Prototype defaults come from the first
declaration. Ordinary and generic methods compete for bare calls, with ordinary
methods winning equal conversion/default-cost ties. Explicit angle brackets
select generic candidates. Nested owners retain their captured arguments.
Original unused methods receive opaque body checking; selected concrete bodies
and defaults activate dependencies through the bounded specialization loop.

Explicit member applications preserve arbitrary receiver expressions, including
function results, nested fields and indexed arrays. Static calls discard receiver
evaluation. Mutating calls and output parameters capture storage coordinates
before user function calls, preventing HLSL copy-in/copy-out from evaluating an
index twice. Imports preserve method visibility and private caller argument types.
Resource handles, aggregate results, arrays, numeric structural patterns and
implicit calls use the ordinary semantic and IR paths after specialization.

See [method values and defaults](../tests/conformance/positive/generic-method-values.slang),
[receiver effects](../tests/conformance/positive/generic-method-receivers.slang),
[generic owners](../tests/conformance/positive/generic-method-owners.slang),
[resource values](../tests/conformance/positive/generic-method-resources.slang)
and [imported methods](../tests/conformance/positive/generic-method-module/main.slang).
Constraints, reference receiver modes,
receiver-dependent defaults and complete polymorphic overload ranking remain
unimplemented. Explicit recursive method specialization has frontend coverage;
GPU recursion validation and execution equivalence are not claimed.

## Interfaces and type constraints

Named interfaces declare required instance/static methods and constructors.
Structs explicitly list their conformances after a colon. Matching method names
alone do not establish conformance.

```slang
interface IValue {
    int read();
    [mutating] void set(int value);
}
struct Item : IValue {
    int value;
    int read() { return value; }
    [mutating] void set(int input) { value = input; }
}
int read<T : IValue>(T item) { return item.read(); }
```

Interfaces can inherit multiple interfaces, including diamonds. Type parameter
constraints accept conjunctions such as `T : ILeft & IRight`. Generic interfaces
retain their argument identity, for example `Read<int>` versus `Read<float>`.
A constraint can refer to earlier parameters: `read<U, T : Read<U>>`. Forward
references in inline constraints report `T032`. `This` in a requirement denotes
the conforming owner; it works in parameters and results.

Direct conformance checks require matching parameter and result types, static
status, receiver mutability, and input/output parameter modes. Missing or
incompatible requirements report `T032`. Type specialization fails with `T033`
when arguments do not satisfy their declared nominal constraints. Nonconforming
function and constructor candidates are removed before selection; remaining
ordinary/generic conversion and default ranking still applies.

Constrained generic bodies are checked on opaque parameter owners exposing only
their inherited requirements, even when no concrete call uses the template.
Method and explicit constructor calls retain the selected requirement through
cached specialization and bind its verified direct implementation. A concrete
overload unavailable on the opaque owner cannot replace that selection. This
applies to free functions, struct owners, generic methods and
generic constructors, including imported interfaces and private caller types.
Validated direct witnesses permit calls across module boundaries without
exposing the concrete method or constructor to ordinary callers. Interfaces do
not become HLSL data structs.

Trailing input defaults on requirements belong to the interface declaration.
For example, `int read(int input = 3)` passes `3` through a constrained call
even if the implementation declares `input = 9`. Defaults keep their module
scope and specialize generic dependencies. Invalid unused defaults are checked.
Constrained `T()` invokes a declared constructor requirement when available;
unconstrained default construction keeps the zero-initialization rules above.
See [witness selection](../tests/conformance/positive/interface-witness-selection.slang)
and [required constructor defaults](../tests/conformance/positive/interface-witness-constructors.slang).

Contextual brace initialization, automatic variables and automatically initialized
fields in user constructors also
retain the selected constructor requirement and its declaration defaults. This
includes nested lists, fixed-array elements, return values and function defaults.
Ordinary concrete constructor calls keep their own overloads and defaults. See
[constructor plans](../tests/conformance/positive/interface-witness-plans.slang).

This implementation covers direct signatures. Default interface bodies, generic
method requirements, associated types, extensions, adapted witnesses (including
numeric conversions, default-argument adaptation and static/instance adaptation),
`where` constraints, operator/conversion constraints, complete constrained
ranking and runtime dispatch remain unsupported. Interface existential variables,
parameters, results and aggregate/resource storage report `T001`. Inheritance is
checked eagerly with a 32-level limit; the oracle can defer errors in unused
cyclic interfaces. See the [interface fixtures](../tests/conformance/positive/interface-methods.slang)
and [generic interface arguments](../tests/conformance/positive/interface-generic-arguments.slang).

## Brace initializer lists

`{...}` uses the type of its declaration, assignment destination, return type,
or selected function parameter. Lists support memberwise resource-valued structs,
direct resource-handle copies, nested data structs, fixed
arrays, scalars, vectors, and matrices. Trailing commas and empty lists are valid.

```slang
float values[3] = {1.0, 2.0}; // final element is zero
float grid[2][3] = {{1.0}, {2.0, 3.0, 4.0}};
float3 splat = {2.0};        // all components are 2
float3 partial = {1.0, 2.0}; // final component is zero
float2x2 rows = {1.0, 2.0};  // first row is 1, second row is 2
```

Struct lists select a user-defined constructor when present. Otherwise they
first try memberwise initialization, including field defaults and visibility.
When that form does not apply, flat values fill nested fields in
declaration order. Arrays also accept flat lists across dimensions. Explicit
nested lists keep their own boundaries; an excess value in an inner list is
an error. Empty lists invoke a user-defined default constructor when available;
otherwise they use zero values and nested struct defaults. Copy a struct
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

Switch selectors require scalar 32-bit, 64-bit, or pointer-sized integers. Labels support integer
literals, const bindings, unary/binary integer operations, conditional
expressions, and scalar integer conversions. The evaluator tracks 32-bit and
64-bit signed and unsigned expression types, retaining unsigned bit patterns.
Duplicate values after selector conversion,
duplicate defaults, and nonconstant labels are rejected. Function calls in
constant expressions and the complete upstream constant evaluator remain absent.
Scalar integer and boolean global constants resolve forward references through
their checked dependencies, including imported declarations. Cyclic integer
initializers are rejected even when the reference occurs in an inactive branch.
Checked case labels are emitted as integer literals after selector conversion,
including labels derived from local static const bindings.

Unsigned 64-bit division, remainder, right shifts, and comparisons preserve
their operand types. This differs from a verified upstream constant-folding
behavior: the pinned `slang-ir-sccp.cpp` dispatches comparisons by their boolean
result type and compares signed `intVal` operands, while `slang-ast-val.cpp`
also uses signed comparisons. Values with bit 63 set can therefore compare
differently after upstream folding. The differential corpus checks both exact
results with separate goldens for seven affected fixed-width/pointer-width words. Full constant-folding
and execution parity remain incomplete.

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

Matrix dimensions one through four, numeric type applications, component
selectors, and explicit storage qualifiers use the same logical row/column
algebra. Buffer storage follows the selected matrix layout.

## Resource operations

`ConstantBuffer<T>` supports scalar, vector, matrix, fixed-array, struct, and
void elements. A handle implicitly loads its element in value contexts, including
initialization, assignment, returns, constructors, input arguments, numeric
operators, conditions, intrinsics, and indexing. For example,
`ConstantBuffer<float> scale; float value = scale;` loads one float. Struct
fields use the usual syntax, such as `parameters.scale`; imported fields retain
their visibility. Buffer contents remain read-only through nested indices,
swizzles, and output arguments; copies into local variables are ordinary values.

Exact handle parameters retain the resource rather than loading its element.
Overloads prefer an exact handle match, then an exact loaded element match over
a numeric conversion. Handle returns, defaults, and arrays preserve this
distinction. A switch requires an integer value: load an integer handle into a
local or use an explicit numeric cast before switching on it. A void element
uses no register or uniform byte layout; loading it still evaluates the source
expression's side effects.

HLSL uses generated storage structs for nonstruct elements. Reflection retains
the source element type and its legacy HLSL byte layout. Matrix qualifiers and
compilation defaults determine storage without changing logical value shapes.
See [resources.md](resources.md#constant-buffer-byte-layout).

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

Resource-valued constant-buffer elements and fields, packed handle arrays, and
byte layout for other targets remain unimplemented.

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
16, 32 or 64 bits and preserves the input's integer sign and width. Supported
integer types are `int16_t`, `uint16_t`, `int`, `uint`, `int64_t`, `uint64_t`, `intptr_t`, and `uintptr_t`.
Boolean scalar/vector arguments promote to `int`. Floating inputs, matrices,
and 8-bit integers are outside these signatures.

The highest-bit search retains signed inputs: negative `int` values search for
the highest zero bit, while unsigned values search for the highest set bit.
The same rule applies to signed and unsigned 64-bit values. A search
without a qualifying bit returns `0xffffffffu`. These calls retain runtime
evaluation, including when used in function defaults. See
[integer-bit-operations.slang](../examples/integer-bit-operations.slang).

## Bit reinterpretation

`asint`, `asuint`, and `asfloat` reinterpret 32-bit numeric components while
preserving scalar, vector, or matrix shape. An input already having the result's
component type passes through unchanged. Boolean components first convert to
`int`, and half components first convert to `float`, matching the upstream
overload selection. These operations differ from numeric casts such as `float(bits)`.

Wide integer inputs can select a 32-bit overload through a numeric conversion.
Variables of fixed-width or pointer-width integer type first convert to `float`;
in-range scalar literals can select `int` or `uint` using their value-aware
conversion cost. For example, `asfloat(1Z)` reinterprets the integer bits of 1,
whereas `asfloat(pointerVariable)` numerically converts that variable to float.
These conversions do not implement a 64-bit bit reinterpretation overload.

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
profile and schema 11's `target_compiler_args` when compiling the HLSL. Native
16-bit mode requires at least 6.2 and applies to all emitted entries and helpers.
A profile selection does not yet validate the
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

`SV_DispatchThreadID`, `SV_GroupID` and `SV_GroupThreadID` accept `uint3` or
`int3`; `SV_GroupIndex` accepts `uint` or `int`. Validation requires a `void`
return, a function body, supported parameter
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

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
- `bool`, `int`, `uint`, `half`, `float`, `double`, and vectors with 2?4 components.
- One-dimensional arrays with a positive decimal literal size.
- `StructuredBuffer<T>` and `RWStructuredBuffer<T>` with indexed element access.

Matrices, textures, samplers, constant buffers, pointers, 64-bit integers,
user-defined generics, and struct constructors are not implemented.

Supported modifiers are `const/static/uniform/groupshared` for globals,
`const/static` for locals, and `const/in/out/inout` for parameters. Conflicting
parameter directions and `const out/inout` are rejected. Global values without
`static/groupshared` are read-only; RW buffer elements remain writable.
Complete modifier rules are not yet implemented. Comma-separated declarations,
default arguments, and aggregate initialization are not supported.

## Expressions and statements

Expressions include numeric and boolean literals, names, scalar and vector
constructors, unary and binary operators, assignment, compound assignment,
function calls, fields, swizzles, indexing, conditionals, and increment/decrement.

Statements include blocks, local declarations, expression statements, `return`,
`if/else`, `while`, `for`, `break`, and `continue`. `switch`, `do/while`, C-style
casts, the comma operator, and statement attributes remain to be ported.

The checker resolves nested scopes, validates conversions and scalar conditions,
and tracks writable locations. It rejects writes to const values, read-only
buffers, and swizzles with repeated components. Overload conversions currently
have equal cost after exact matches. Full overload ranking, definite assignment,
constant evaluation, uniformity analysis, and capability checking are absent.

Syntax nesting and expression trees are limited to 128 levels to prevent stack
overflow on malformed or unusually deep inputs.

## Intrinsics

The current built-in signatures cover `abs`, `sin`, `cos`, `tan`, `sqrt`, `rsqrt`,
`floor`, `ceil`, `frac`, `exp`, `exp2`, `log`, `log2`, `saturate`, `min`, `max`,
`pow`, `clamp`, `lerp`, `dot`, `length`, `normalize`, `all`, `any`,
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
semantics, and a valid thread-group size. Other shader stages, semantics, and
shader attributes have not been implemented.

## Reflection

JSON reflection reports entry points, stages, thread-group sizes, global names
and type names, and read/read_write access for structured buffers. It does not
provide binding indices, register spaces, byte offsets, strides, or layout.
Applications must not treat this JSON as the upstream compiler's ABI reflection.

# Resources and register bindings

The native compiler supports constant buffers, structured buffers, byte-address
buffers, 1D/2D/3D/cube textures, and ordinary/comparison samplers. Each supported
resource handle can appear in a fixed array. Index the array before using the
selected handle's operations. See [language.md](language.md) for those operations.

`ConstantBuffer<T>` accepts supported data types and void. Nonstruct elements
use generated HLSL storage structs; reflection describes the original element
and binding. Value contexts load the element, while handle parameters and
returns preserve the resource. Void elements and empty structs consume no
constant-buffer register. Their source expressions still evaluate when used.

## Explicit registers

Global shader resources accept HLSL register annotations:

```slang
struct Parameters { float scale; };
ConstantBuffer<Parameters> parameters : register(b0, space1);
Texture2D<float4> images[2] : register(t0, space1);
SamplerState imageSampler : register(s0, space1);
RWStructuredBuffer<float4> output : register(u0, space1);
```

| Resource | Register class |
| --- | --- |
| ConstantBuffer | `b` |
| Read-only textures, structured buffers, byte-address buffers | `t` |
| Writable textures, structured buffers, byte-address buffers | `u` |
| SamplerState, SamplerComparisonState | `s` |

Class and space prefixes are lowercase. Indices are decimal integers between
zero and 2,147,483,647. `register(t01, space02)` normalizes to
`register(t1, space2)`. Omitting the space means zero; emitted HLSL omits
`space0`. A resource component mask such as `register(t0.x)` is accepted and
ignored, matching upstream resource binding behavior.

Each declarator owns its annotation, including declarations separated by commas.
Aliases resolve before the register class is checked. Imports and includes retain
the declared class, index, and space when target identifiers change. Register
tokens are annotation data and do not refer to variables named `t0` or `space1`.
Repeated identical register annotations are accepted; conflicting ones fail.

Fixed resource arrays occupy a contiguous range. A declaration such as
`Texture2D<float> images[2][3] : register(t4, space2)` uses six registers,
`t4` through `t9`, in space two. A range must fit entirely inside the supported
index range. The compiler computes its count without expanding array elements.
Dimensions can use compile-time scalar expressions. A zero dimension consumes
no registers, even in a nested array, and is recognized before range
multiplication. Explicit register classes are still checked on empty arrays.

Explicit overlaps within the same class and space produce warning `T023`.
Different classes and spaces have independent ranges. Warnings remain available
from successful API calls and `--check`. The compiler preserves the specified
bindings, so DXC can reject HLSL when overlapping resources are used. Upstream
also accepts these source declarations with a warning.

## Reflection

Reflection schema version 10 includes the allocated binding on each global shader resource:

```json
{
  "name": "images",
  "target_name": "images",
  "type_name": "Texture2D<float4>[2]",
  "resource_access": "read",
  "binding": { "class": "t", "index": 0, "space": 1, "count": 2, "explicit": true }
}
```

The `explicit` flag is true for a source annotation and false for automatic
assignment. Numeric/aggregate uniforms and private storage omit their individual
`binding`. Ordinary global uniform values share a constant buffer whose allocated
register is reported by the top-level `global_uniform_binding` field:

```json
{
  "global_uniform_binding": { "class": "b", "index": 1, "space": 0, "count": 1, "explicit": false }
}
```

This field is absent when there are no ordinary global uniform values. It remains
available during `--check`.

Generated constructor helpers and private local-static storage do not appear
as source globals. [resource-bindings.slang](../examples/resource-bindings.slang)
provides a complete shader example.

## Automatic allocation

Resources without register annotations receive the first contiguous free range
in space zero. All explicit ranges are reserved before any automatic assignment.
Classes and spaces have independent occupied intervals. An array requires one
contiguous interval; a later smaller resource can use a gap the array skipped.

```slang
Texture2D<float> images[2];             // t2 through t3
Texture2D<float> fixed : register(t1);  // t1
Texture2D<float> image;                 // t0
```

Allocation follows declaration order within the root module, then module
discovery order for imports. Unused resources still receive slots, matching
upstream parameter layout. Ordinary global uniforms reserve one implicit
constant-buffer slot before resource allocation. The HLSL emitter groups these
values into `cbuffer _slang_globals` with that explicit register. For example,
an explicit `ConstantBuffer` at `b0` moves this shared buffer to `b1`, and the
next automatic `ConstantBuffer` uses `b2`. This keeps the emitted binding and
reflection consistent even when explicit resources occupy the first slots.
Numeric values, aggregates, fixed arrays, and imported uniforms share this buffer;
their byte packing follows the reflected legacy HLSL layout. Static and groupshared variables
do not consume shader resource bindings or implicit uniform slots. Allocation
state belongs to one compilation and does not persist across API calls.

The HLSL emitter writes every selected resource register explicitly, so target
compilation preserves the indices reported by reflection. Exhausted or
overflowing contiguous ranges produce `T022` without expanding array elements.

## Constant-buffer byte layout

Global uniform values, direct `cbuffer` declarations, and `ConstantBuffer` elements expose `uniform_layout` with
an `offset` and a canonical `type_name`. Look up that name in the top-level
`uniform_types` table. Each table entry reports `size` and `alignment`, plus
`fields` for structs, `array_stride` and `element_type` for arrays, or
`matrix_stride` for matrices. Field offsets are relative to their containing
struct. Array elements share one referenced layout instead of expanding into
individual records.

```json
{
  "uniform_layout": { "offset": 16, "type_name": "Parameters" },
  "uniform_types": {
    "Parameters": {
      "size": 8,
      "alignment": 16,
      "fields": [
        { "name": "scale", "offset": 0, "type_name": "float" },
        { "name": "bias", "offset": 4, "type_name": "float" }
      ]
    },
    "float": { "size": 4, "alignment": 4 }
  }
}
```

For a global value, `offset` is measured from the shared global uniform buffer.
For `ConstantBuffer<T>` and arrays of those handles, it is zero and `type_name`
identifies `T`; the resource binding still describes the handle array. The
top-level `global_uniform_size` gives the occupied bytes in the shared buffer.
Unused uniforms retain their offsets. Root-module values precede imported values
in both the emitted buffer and reflection. Private storage and structured-buffer
elements do not receive constant-buffer layouts.

These layouts follow legacy DXC packing: 32-bit scalars use four bytes, doubles
use eight, and vectors align to their components. A value that would cross a
16-byte boundary starts at the next boundary. Structs and arrays start on
16-byte boundaries, but their final size is not rounded up. For example,
`float2 values[2]` occupies 24 bytes with a stride of 16; a following float can
occupy its remaining tail space. `size` is the occupied extent, so host buffer
allocation must apply the alignment required by the graphics API.

Empty arrays and structs consume no uniform bytes and do not create a shared
buffer by themselves. Empty constant-buffer elements consume no buffer register.
The source type table retains zero sizes and array dimensions. HLSL storage
omits empty fields and reconstructs their values, so DXC's empty-struct alignment
cannot move adjacent nonempty fields.

Matrices default to column-major storage. `row_major` and `column_major`
qualify matrix declarations and arrays of matrices; aliases preserve their
qualifier. `-matrix-layout-row-major` and `-matrix-layout-column-major` select
the compilation default, also available as `CompileOptions.matrix_major`.
The emitter records that default with an HLSL pragma. `matrix_major` reports
the selected order for each matrix layout, and `matrix_stride` is the byte
distance between stored rows or columns. Row-major layouts use distinct
keys such as `row_major float2x3` in the shared type table.

A single-column column-major matrix packs like a vector in legacy DXC.
Single-row row-major matrices still start on a 16-byte boundary; a following
scalar can reuse the last row's tail space. Native `half` values use four-byte storage
because the current HLSL pipeline does not enable native 16-bit types. Slang
2026.21's reflection instead reports two-byte half components and 16-byte
alignment for single-column matrices; those cases are checked against DXC's
actual native buffer definitions rather than claimed as upstream layout parity.

Offsets, sizes, and strides use checked 64-bit arithmetic. Overflow reports
`I002` at the global declaration without expanding array elements. Referenced
types are visited and cached once. Mixed resource-valued aggregate layouts,
16-bit storage, other buffer packing modes, and
target buffer-size limit validation remain absent. Mixed resource aggregates
omit shared-buffer byte metadata until their layout model is implemented.
See [uniform-layout.slang](../examples/uniform-layout.slang) for a complete example.

Matrix arrays with a storage order different from the compilation default are
loaded as logical values before aggregate copies or function arguments. Constant
buffers use flattened matrix declarations; structured-buffer loads reconstruct
the values from their components. Dynamic indices are evaluated once, and writable
locations remain available for assignments, output parameters, and atomics.
Materialization is bounded to 65,536 generated nodes and reports `I004` if it
exceeds that limit. Explicit matrix element types such as
`StructuredBuffer<row_major float2x3>` use a generated storage struct in HLSL;
source reflection retains the resource's matrix type.

Whole-array constant-buffer loads reconstruct logical elements before copies
and function arguments. A helper receives the selected handle once, preserving
side effects in its index or producing function. Direct array reads access only
the selected element and do not expand the whole array. The same materialization
limit applies to these copies. Scalar, vector, and matrix loads use the generated
storage field directly; struct loads explicitly convert to their value type.

Direct `cbuffer` fields and ordinary struct fields accept `: packoffset(c2.y)`. Register indices count
16-byte slots; `x`, `y`, `z`, and `w` select byte offsets 0, 4, 8, and 12.
Omitting the component selects `x`. Reflection preserves the explicit byte
offset, and HLSL emits a normalized `packoffset`. All explicit fields are placed
before automatic fields, which start after the largest occupied explicit end.
Explicit fields may overlap. Static members ignore `packoffset` because they
have private storage. The generated value struct carries no HLSL packing
annotations; packing is applied to direct buffer fields. Packed structs and
arrays of packed values are flattened into buffer declarations at their
reflected absolute offsets. Generated load functions reconstruct their values
for field access, copies, defaults, and function arguments. Packed
`ConstantBuffer<T>` parameters lower to value parameters. Source reflection
continues to describe the original constant-buffer handle and its binding.
For a packed ordinary global, `target_name` identifies its load function;
resource globals retain their physical buffer name.

Explicit placement bypasses the automatic alignment rules. As in upstream,
the frontend preserves offsets that DXC may reject, such as a `float2` at
`c0.w` or an array at `c1.y`. DXC validation is required to establish HLSL
validity. Packoffsets on standalone uniforms, function parameters, and locals
remain unsupported. Arrays of packed constant-buffer handles and materialization
beyond 65,536 value nodes report `I004`. Indices use the resource parser's
bounded decimal range; component masks require exactly one supported component.

Slang 2026.21's HLSL drops packing inside nested structs and arrays, even though
its reflection retains those offsets. Native HLSL preserves the reflected
offsets. Tests check both physical buffer definitions against separate expected
layouts; these cases do not establish shader execution parity with upstream.

## Current limits

Vulkan binding attributes, packed constant-buffer handle arrays,
other target layouts, and the upstream reflection API remain to be implemented.
Resource register annotations select bindings on global resources. On data-only
global uniforms, `b`, `t`, `u`, and `s` annotations are accepted and ignored;
the values still use the shared global uniform buffer. Struct fields, parameters,
function results, static storage, and groupshared storage accept ignored
annotations, including legacy `register(c2.y)`. Legacy `c` annotations cannot
bind standalone global shader parameters or resources, and local declarations
cannot carry register annotations. Mixed resource-valued aggregates still lack
register layout. Global
attributes such as `[allow("overlapping-bindings")]` are also unsupported.

Slang can also wrap overflowing
register indices into invalid HLSL numbers; the native parser reports `S001`
instead. Wrong classes report `T022`, and malformed register syntax reports
`S001`. Diagnostic codes are native codes, independent of upstream numeric IDs.

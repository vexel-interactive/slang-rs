# Resources and register bindings

The native compiler supports constant buffers, structured buffers, byte-address
buffers, 1D/2D/3D/cube textures, and ordinary/comparison samplers. Each supported
resource handle can appear in a fixed array. Index the array before using the
selected handle's operations. See [language.md](language.md) for those operations.

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

Explicit overlaps within the same class and space produce warning `T023`.
Different classes and spaces have independent ranges. Warnings remain available
from successful API calls and `--check`. The compiler preserves the specified
bindings, so DXC can reject HLSL when overlapping resources are used. Upstream
also accepts these source declarations with a warning.

## Reflection

Reflection schema version 6 includes the explicit binding on each bound global:

```json
{
  "name": "images",
  "target_name": "images",
  "type_name": "Texture2D<float4>[2]",
  "resource_access": "read",
  "binding": { "class": "t", "index": 0, "space": 1, "count": 2 }
}
```

Unbound globals omit `binding`; their slots still require automatic allocation
by the target compiler. This reflection does not describe those allocated slots.
Generated constructor helpers and private local-static storage do not appear
as source globals. [resource-bindings.slang](../examples/resource-bindings.slang)
provides a complete shader example.

## Current limits

Automatic allocation, default-space options, Vulkan binding attributes, direct
`cbuffer` declarations, uniform byte offsets, packing, strides, and the upstream
reflection API remain to be implemented. Register annotations currently require
global resources. Numeric/aggregate uniforms, struct fields, parameters, local
variables, static storage, and groupshared storage cannot receive them. Global
attributes such as `[allow("overlapping-bindings")]` are also unsupported.

These context limits are stricter than upstream in some cases. Slang accepts
numeric uniform annotations and supplies implicit uniform layout; this compiler
reports `T022` until that layout is implemented. Slang can also wrap overflowing
register indices into invalid HLSL numbers; the native parser reports `S001`
instead. Wrong classes report `T022`, and malformed register syntax reports
`S001`. Diagnostic codes are native codes, independent of upstream numeric IDs.

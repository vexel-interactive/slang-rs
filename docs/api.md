# Rust API

`slang-compiler` is the primary entry point. Internal crates are also available,
but their models will evolve as the port grows.

```rust
use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let compilation = compile_source(
        "example.slang",
        "float twice(float x) { return x * 2.0; }",
        &CompileOptions::default(),
        &MemoryIncludes::default(),
    )?;
    println!("{}", compilation.artifact.unwrap());
    Ok(())
}
```

Compilation returns `Result<Compilation, CompileError>`. `Compilation` contains
a text artifact, typed IR, declaration reflection, a source map, and warnings.
Reflection schema version 10 reports `compute`, `vertex`, or `fragment` stages;
`thread_group_size` is present only for compute entries. The Rust model uses
`slang_sema::ShaderStage` and `Option<[u32; 3]>` for these fields.
`EntryPointInfo` and `GlobalInfo` retain the source `name` and add `target_name`
for HLSL identifier renaming. Use `target_name` as the target compiler's entry
identifier; for common names such as `main` it is identical to `name`.
`EntryPointInfo.target_profile` gives the selected HLSL profile. Pass it to DXC's
`-T` option. Compute derivatives require `cs_6_6` or higher; callers must use the
reflected profile after an automatic upgrade. The optional `CompileOptions.profile`
accepts a parsed `ShaderProfile`, for example `Some("cs_6_6".parse()?)`.
Supported profile spellings are `cs_6_N`, `vs_6_N`, and `ps_6_N` for minor versions
zero through nine. Parsing a profile does not establish support for every feature
of that shader model; only implemented requirements are checked.
`Reflection.modules` describes each loaded module's identity, imports, and
member source IDs. `GlobalInfo.module` identifies imported globals' owner;
root globals omit it. Imported struct type names are canonical linked names.
`GlobalInfo.binding: Option<ResourceBinding>` reports global shader resources.
Its JSON object contains `class` (`b`, `t`, `u`, or `s`), `index`, `space`, and
`count`, and `explicit`. Array dimensions multiply into `count`; automatic
bindings use space zero. `explicit` distinguishes source annotations from
automatic assignment. Numeric uniforms and private storage omit `binding`.
`Reflection.global_uniform_binding: Option<ResourceBinding>` reports the shared
constant-buffer register for ordinary global uniform values. It is absent when
there are no such values; static and groupshared storage are excluded. The HLSL
emitter uses this register explicitly, and `check_only` preserves it in reflection.
`GlobalInfo.uniform_layout: Option<UniformVariableLayout>` holds an offset and
canonical value type name for global uniforms and `ConstantBuffer` elements.
Packed ordinary globals use generated load functions; their `target_name`
identifies that function. Packed resource globals retain the target buffer name.
`Reflection.uniform_types` maps these names to shared `UniformTypeLayout` entries:
occupied byte size, alignment, struct field offsets, array strides and element
types, matrix strides, and `matrix_major` (`row` or `column`). A row-major matrix
or matrix array uses a layout key such as `row_major float2x3`; its algebraic
value type remains `float2x3`. `Reflection.global_uniform_size` reports
the occupied size of the shared buffer. Private storage and structured-buffer
elements omit this metadata. These models are reexported from `slang-compiler`.
`ConstantBuffer` globals retain their source scalar, vector, matrix, array,
struct, or void element in reflection even when HLSL uses generated storage.
Void elements omit both `binding` and `uniform_layout`. In `slang-sema`,
`Type::ConstantBuffer { element, matrix_major }` stores the value type and any
matrix storage qualifier that differs from the compilation default.
See [resources.md](resources.md#constant-buffer-byte-layout) for legacy HLSL rules,
host allocation requirements, native half storage, and the remaining limits.
The binding types are reexported from `slang-compiler` and defined
in `slang-layout`. See [resources.md](resources.md) for the checked limits.
Overlapping explicit ranges produce warning `T023` in `Compilation.diagnostics`;
compilation and `check_only` still succeed. Such HLSL may fail target validation.
With `check_only: true`, the artifact is `None`. Errors retain diagnostics and
source files; their `Display` implementation reports source locations and text.

`CompileOptions.matrix_major` selects the default matrix storage order using
`MatrixMajor::Column` (the native default) or `MatrixMajor::Row`. This option
controls type checking, HLSL emission, and byte layout together; the selected
default is recorded in `Reflection.matrix_major`. Source `row_major` and
`column_major` qualifiers override it, including through aliases. For direct
semantic-crate callers, `check_program_with_matrix_major` records the same
choice in `CheckedModule`, which the IR lowerer preserves.

`compile_file(path, options)` loads a UTF-8 file and uses the filesystem resolver.
Its `include_paths` option controls include, import, and module-member lookup. `compile_source` uses the
resolver supplied by the caller; it does not override that resolver with the
`include_paths` field.

The native API defaults `__SLANG__` to one and preserves an explicit caller
definition. `slang-preprocessor` supplies `__LINE__` and `__FILE__` from the
initiating source location. Those builtins can be redefined or removed within
the current preprocessing environment; redefining one emits warning `P010`.

```rust
use slang_compiler::{CompileOptions, compile_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let options = CompileOptions {
        entry_point: Some("main".into()),
        check_only: true,
        ..CompileOptions::default()
    };
    let compilation = compile_file("examples/vector-add.slang", &options)?;
    assert!(compilation.artifact.is_none());
    Ok(())
}
```

## In-memory includes

```rust
use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let includes = MemoryIncludes {
        files: [("constants.slang".into(), "static const float scale = 2.0;".into())].into(),
    };
    let compilation = compile_source(
        "main.slang",
        "#include \"constants.slang\"\nfloat f() { return scale; }",
        &CompileOptions::default(),
        &includes,
    )?;
    assert!(compilation.artifact.is_some());
    Ok(())
}
```

Implement `IncludeResolver::resolve` to integrate a virtual filesystem. Return a
stable resource name and its text; `#pragma once` and module caching rely on that identity.
Resolvers can enforce their own directory or resource access policy.

## In-memory modules

The same resolver supplies `import` and `__include` sources. `MemoryIncludes`
uses normalized file requests as exact map keys:

```rust
use slang_compiler::{CompileOptions, MemoryIncludes, compile_source};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = MemoryIncludes {
        files: [("math-utils.slang".into(),
            "module math_utils; public float twice(float x) { return x * 2.0; }".into())].into(),
    };
    let compilation = compile_source(
        "main.slang",
        "import math_utils; float run() { return twice(2.0); }",
        &CompileOptions::default(),
        &files,
    )?;
    assert_eq!(compilation.reflection.modules.len(), 2);
    Ok(())
}
```

For phase-level integrations, `slang_module::load` accepts a root `SourceId`
already owned by the mutable `SourceMap`, a resolver, and preprocessor options.
It returns a linked `Program`; pass that to `slang_sema::check_program`.
Direct `slang_sema::check` handles a single resolved AST and rejects unresolved
imports or member directives. Loading a program owns no process-wide cache.
Successful `CheckedModule` values expose semantic warnings in `diagnostics` and
resource ranges in `resource_bindings`, keyed by declaration byte spans, and the
shared uniform buffer register in `global_uniform_binding`. IR lowering adds the
constant-buffer byte layouts used by emission and declaration reflection.

## API compatibility

The COM-style interfaces from `slang.h`, C ABI, session/component model,
separately compiled components, specialization, and binary blobs are not yet implemented.
Version 0.1.0 does not promise stable ABI or internal IR formats. Full upstream
public API compatibility is a separate required part of the porting plan.

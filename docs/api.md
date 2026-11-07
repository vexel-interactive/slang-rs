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
Reflection schema version 5 reports `compute`, `vertex`, or `fragment` stages;
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
With `check_only: true`, the artifact is `None`. Errors retain diagnostics and
source files; their `Display` implementation reports source locations and text.

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

## API compatibility

The COM-style interfaces from `slang.h`, C ABI, session/component model,
separately compiled components, specialization, and binary blobs are not yet implemented.
Version 0.1.0 does not promise stable ABI or internal IR formats. Full upstream
public API compatibility is a separate required part of the porting plan.

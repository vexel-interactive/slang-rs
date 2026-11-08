# slang-rs

A native Rust implementation of the [Slang shading language](https://github.com/shader-slang/slang).
The goal is a complete rewrite that preserves Slang's language, semantics,
compilation targets, reflection, and tools. The Rust workspace separates the
compiler into clearly defined phases.

**The full rewrite is not complete.** The current implementation compiles a
limited compute, vertex, and fragment shader subset to HLSL. It includes preprocessing, parsing, type
checking, native module loading, type aliases, aggregate initialization and field defaults,
typed structured IR, scalar SSA constant propagation, resource register allocation,
a Rust API, and a command-line compiler.
Complete module sessions, user generics, interfaces, autodiff, advanced resources and most intrinsics, binary
backends, complete reflection, the upstream C ABI, and LSP remain to be ported.
See the [compatibility matrix](docs/compatibility.md) for exact status.

The production compiler is implemented in Rust. The original compiler is kept
separately as a development oracle. Building this workspace does not require
C++, CMake, LLVM, a Vulkan SDK, or an installed Slang compiler.

## Quick start

Rust **1.99.1** is pinned in `rust-toolchain.toml`. It was the latest stable
release when the project started on November 7, 2026.

```powershell
rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
rustup run 1.99.1 cargo build --release --locked
./target/release/slangc-rs examples/vector-add.slang -entry main -target hlsl -o vector-add.hlsl
```

Windows executables have the `.exe` extension. If PATH contains an older
standalone Rust installation, use [tools/check.ps1](tools/check.ps1), which
explicitly selects the pinned compiler and tools. See [build instructions](docs/building.md).

```powershell
./target/release/slangc-rs examples/control-flow.slang --check
./target/release/slangc-rs examples/vector-add.slang --dump-ir -o vector-add.ir.json
./target/release/slangc-rs examples/vector-add.slang --reflection reflection.json
./target/release/slangc-rs examples/modules/main.slang -entry main -o modules.hlsl
```

HLSL is a textual output. Applications must compile it for their graphics API,
bind resources, and dispatch work before running it on a GPU. This project does
not yet provide a GPU runtime.

## Project layout

```text
crates/
  slang-source/        source files, spans, diagnostics
  slang-layout/        shared resource binding and layout models
  slang-lexer/         tokens and lexical analysis
  slang-preprocessor/  macros, includes, conditional compilation
  slang-syntax/        AST, declarations, expressions, statements
  slang-module/        imports, module members, dependency graph, name linking
  slang-sema/          names, types, overloads, shader validation
  slang-ir/            structured IR, scalar SSA, and lowering
  slang-codegen/       backend dispatch and HLSL generation
  slang-compiler/      Rust API, filesystem, reflection, pipeline
  slang-cli/           command-line arguments and artifact output
docs/                  architecture, builds, API, language, porting
examples/              small Slang source examples
tools/                 validation and upstream inventory tools
tests/conformance/     differential fixtures and their manifest
```

Each crate owns one responsibility. Dependencies run from the CLI and API down
to compiler phases, without cycles. The workspace forbids `unsafe` code.
`rustfmt` formats the source, and CI treats Clippy warnings as errors.

## Documentation

- [Building and toolchains](docs/building.md)
- [Architecture and subsystem boundaries](docs/architecture.md)
- [Supported language subset](docs/language.md)
- [Modules and access control](docs/modules.md)
- [Resources and register bindings](docs/resources.md)
- [Rust API](docs/api.md)
- [Tests and comparison with upstream](docs/testing.md)
- [Compatibility and remaining work](docs/compatibility.md)
- [Full porting plan](docs/porting.md)
- [Contribution guidelines](CONTRIBUTING.md)

The upstream source baseline is commit
`ae4e57957a0adad3061b069b0e0eec8217779338`. Reference metadata is recorded in
[upstream.lock.json](upstream.lock.json) and the [source inventory](docs/upstream-inventory.json).
The standalone comparison oracle is release `v2026.21`.

## License

Apache-2.0 WITH LLVM-exception, matching the current upstream compiler.
See [LICENSE](LICENSE) and [NOTICE](NOTICE). Slang and its upstream project retain
their respective ownership. `slang-rs` is an independent project by Ankerin.

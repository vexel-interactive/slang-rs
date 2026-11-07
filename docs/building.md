# Building

Requirements: Rust 1.99.1, a platform linker, and crates.io access for the first
build. Windows uses MSVC Build Tools; Linux and macOS use the system C linker.
The project compiles Rust source and does not build the C++ Slang compiler.

```text
rustup toolchain install 1.99.1 --profile minimal --component rustfmt --component clippy
rustup run 1.99.1 cargo build --workspace --locked
rustup run 1.99.1 cargo build --release --locked
```

`Cargo.lock` is checked in. Release builds enable thin LTO and a single codegen
unit. No nightly features or unstable rustfmt options are required.

## Multiple Rust installations on Windows

When several Rust installations are present, ensure Cargo, Clippy, and rustc
use the same toolchain. For reproducible Windows checks:

```powershell
powershell -ExecutionPolicy Bypass -File tools/check.ps1
```

The script selects PATH, RUSTC, and RUSTDOC within its process and restores them
on exit. It does not change global Rust configuration or system settings.
To select the compiler explicitly in a shell:

```powershell
$taskRustBin = Split-Path (rustup which --toolchain 1.99.1 rustc)
$env:PATH = "$taskRustBin;$env:PATH"
$env:RUSTC = Join-Path $taskRustBin 'rustc.exe'
$env:RUSTDOC = Join-Path $taskRustBin 'rustdoc.exe'
cargo build --release --locked
```

## API documentation

```text
rustup run 1.99.1 cargo doc --workspace --no-deps --locked
```

Open `target/doc/slang_compiler/index.html`. In a mixed Windows toolchain
environment, select PATH, RUSTC, and RUSTDOC as shown above first.

## Checks

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

The CI workflow configures these checks for Windows, Linux, and macOS using the
pinned toolchain. A workflow definition does not imply its remote jobs have
already run.

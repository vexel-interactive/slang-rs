param([switch]$Release)

$ErrorActionPreference = 'Stop'
$taskRoot = Split-Path $PSScriptRoot -Parent
Push-Location -LiteralPath $taskRoot
$taskPreviousPath = $env:PATH
$taskPreviousRustc = $env:RUSTC
$taskPreviousRustdoc = $env:RUSTDOC
try {
    $taskRustc = rustup which --toolchain 1.99.1 rustc
    if ($LASTEXITCODE -ne 0) { throw 'Install Rust 1.99.1 with rustfmt and clippy first.' }
    $taskRustBin = Split-Path $taskRustc
    $env:PATH = "$taskRustBin;$env:PATH"
    $env:RUSTC = $taskRustc
    $env:RUSTDOC = Join-Path $taskRustBin 'rustdoc.exe'
    rustc --version
    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'rustfmt failed' }
    cargo clippy --workspace --all-targets --locked -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
    cargo test --workspace --locked
    if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
    cargo doc --workspace --no-deps --locked
    if ($LASTEXITCODE -ne 0) { throw 'rustdoc failed' }
    if ($Release) {
        cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    }
} finally {
    $env:PATH = $taskPreviousPath
    $env:RUSTC = $taskPreviousRustc
    $env:RUSTDOC = $taskPreviousRustdoc
    Pop-Location
}

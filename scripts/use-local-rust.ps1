# Activate the ignored project-local Rust installation for this shell only.
$projectRoot = Split-Path -Parent $PSScriptRoot
$env:CARGO_HOME = Join-Path $projectRoot 'local-analysis/toolchain/cargo'
$env:RUSTUP_HOME = Join-Path $projectRoot 'local-analysis/toolchain/rustup'
$localRustBin = Join-Path $env:CARGO_HOME 'bin'
if (-not (Test-Path -LiteralPath (Join-Path $localRustBin 'cargo.exe'))) {
    throw 'Project-local Rust is not installed. Use a normal Rust installation or see STATUS.md.'
}
$env:PATH = "$localRustBin;$env:PATH"
$localGnuTools = Join-Path $env:RUSTUP_HOME 'toolchains/stable-x86_64-pc-windows-gnu/lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained'
if (Test-Path -LiteralPath (Join-Path $localGnuTools 'dlltool.exe')) {
    $env:PATH = "$localGnuTools;$env:PATH"
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $localGnuTools 'x86_64-w64-mingw32-gcc.exe'
}
$localAssemblerBin = Join-Path $projectRoot 'local-analysis/toolchain/msys2/mingw64/bin'
if (Test-Path -LiteralPath (Join-Path $localAssemblerBin 'as.exe')) {
    $env:PATH = "$localAssemblerBin;$env:PATH"
}

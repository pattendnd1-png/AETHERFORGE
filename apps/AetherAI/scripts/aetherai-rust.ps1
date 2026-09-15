$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$HostTriple = 'x86_64-pc-windows-gnu'
$Toolchain = Join-Path $Root "tools\rust\$HostTriple"
$Bin = Join-Path $Toolchain 'bin'
$Cargo = Join-Path $Bin 'cargo.exe'
$MingwBin = Join-Path $Toolchain 'lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained'
if (-not (Test-Path -LiteralPath $Cargo -PathType Leaf)) {
    [Console]::Error.WriteLine('AETHERAI_BUNDLED_RUST=UNAVAILABLE')
    [Console]::Error.WriteLine("AETHERAI_EXPECTED_CARGO=$Cargo")
    exit 127
}
$env:PATH = "$Bin;$MingwBin;$env:PATH"
$env:CARGO_HOME = Join-Path $Root '.aetherai\cargo-home'
$env:SYSROOT = $Toolchain
$env:RUSTC = Join-Path $Bin 'rustc.exe'
$env:RUSTDOC = Join-Path $Bin 'rustdoc.exe'
$env:RUSTFLAGS = "--sysroot=$Toolchain"
$env:RUSTDOCFLAGS = "--sysroot=$Toolchain"
$env:RUSTFMT = Join-Path $Bin 'rustfmt.exe'
$env:CLIPPY_DRIVER = Join-Path $Bin 'clippy-driver.exe'
$env:CARGO_NET_OFFLINE = 'true'
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = Join-Path $MingwBin 'x86_64-w64-mingw32-gcc.exe'
New-Item -ItemType Directory -Force -Path $env:CARGO_HOME | Out-Null
& $Cargo @args
exit $LASTEXITCODE

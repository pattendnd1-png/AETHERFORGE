$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$Rust = Join-Path $PSScriptRoot 'aetherai-rust.ps1'
$Toolchain = Join-Path $Root 'tools\rust\x86_64-pc-windows-gnu'
$Cargo = Join-Path $Toolchain 'bin\cargo.exe'
$Rustc = Join-Path $Toolchain 'bin\rustc.exe'
$Rustfmt = Join-Path $Toolchain 'bin\rustfmt.exe'
$Clippy = Join-Path $Toolchain 'bin\cargo-clippy.exe'
$Gcc = Join-Path $Toolchain 'lib\rustlib\x86_64-pc-windows-gnu\bin\self-contained\x86_64-w64-mingw32-gcc.exe'
$Report = Join-Path $Root 'AetherAI-v0.2.2-VERIFY.txt'
$failed = $false
function Check-Path([string]$Path,[string]$Marker){ if(Test-Path -LiteralPath $Path){Write-Output "$Marker=PASS"}else{Write-Output "$Marker=FAIL";$script:failed=$true} }
Check-Path $Cargo 'AETHERAI_WINDOWS_CARGO'; Check-Path $Rustc 'AETHERAI_WINDOWS_RUSTC'; Check-Path $Rustfmt 'AETHERAI_WINDOWS_RUSTFMT'; Check-Path $Clippy 'AETHERAI_WINDOWS_CLIPPY'; Check-Path $Gcc 'AETHERAI_WINDOWS_MINGW_GCC'; Check-Path (Join-Path $Root 'Cargo.lock') 'AETHERAI_CARGO_LOCK'; Check-Path (Join-Path $Root '.cargo\config.toml') 'AETHERAI_VENDOR_CONFIG'; Check-Path (Join-Path $Root 'resources\inference\runtime-manifest.json') 'AETHERAI_RUNTIME_MANIFEST'
if($failed){Write-Output 'AETHERAI_WINDOWS_SELF_CONTAINED_RUST=FAIL';exit 1}
$env:CARGO_NET_OFFLINE='true'
& $Rust run --offline -p xtask -- verify
$rc=$LASTEXITCODE
if(Test-Path -LiteralPath $Report){ Add-Content -LiteralPath $Report -Value 'AETHERAI_PLATFORM=windows-x86_64'; Add-Content -LiteralPath $Report -Value 'AETHERAI_BUNDLED_RUST_WINDOWS=PASS'; Add-Content -LiteralPath $Report -Value 'AETHERAI_WINDOWS_MINGW=PASS'; Add-Content -LiteralPath $Report -Value 'AETHERAI_VENDOR_OFFLINE=PASS'; if($rc -eq 0){Add-Content -LiteralPath $Report -Value 'AETHERAI_V0_2_2_WINDOWS_VERIFY=PASS'}else{Add-Content -LiteralPath $Report -Value "AETHERAI_V0_2_2_WINDOWS_VERIFY=FAIL:$rc"} }
exit $rc

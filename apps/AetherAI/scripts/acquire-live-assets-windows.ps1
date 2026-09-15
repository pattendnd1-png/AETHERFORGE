$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$RuntimeRoot = Join-Path $Root 'tools\inference'
$ModelRoot = Join-Path $Root 'models'
$RuntimeArchive = 'llama-b10649-bin-win-vulkan-x64.zip'
$RuntimeUrl = 'https://github.com/ggml-org/llama.cpp/releases/download/b10649/llama-b10649-bin-win-vulkan-x64.zip'
$RuntimeSha = '922bbc6cff5880106d1f5cd2451ce19f3c5e657acaf8aadb828501a07e9939d4'
$ModelFile = 'Qwen3-0.6B-Q4_K_M.gguf'
$ModelUrl = 'https://huggingface.co/Qwen/Qwen3-0.6B-GGUF/resolve/1208e45d782fe18602c5eaf10e5758d5b0f24c03/Qwen3-0.6B-Q4_K_M.gguf'
$ModelSha = 'b0638f08417a2d3c8652760462eb5407c6e30173cf9608ad0820757a281eea0e'
New-Item -ItemType Directory -Force -Path (Join-Path $RuntimeRoot 'downloads'), $ModelRoot | Out-Null
function Get-Verified([string]$Url,[string]$Path,[string]$Sha) {
    if ((Test-Path -LiteralPath $Path -PathType Leaf) -and ((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() -eq $Sha)) { return }
    $Part = "$Path.part"
    & curl.exe --fail --location --retry 3 --continue-at - --output $Part $Url
    if ($LASTEXITCODE -ne 0) { throw "curl failed with exit $LASTEXITCODE" }
    $Actual = (Get-FileHash -LiteralPath $Part -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($Actual -ne $Sha) { Remove-Item $Part -Force -ErrorAction SilentlyContinue; throw "SHA-256 mismatch: $Actual" }
    Move-Item -LiteralPath $Part -Destination $Path -Force
}
$Archive = Join-Path (Join-Path $RuntimeRoot 'downloads') $RuntimeArchive
Get-Verified $RuntimeUrl $Archive $RuntimeSha
$InstallDir = Join-Path $RuntimeRoot 'llama-b10649-windows-vulkan-x86_64'
Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Expand-Archive -LiteralPath $Archive -DestinationPath $InstallDir -Force
$Server = Get-ChildItem -LiteralPath $InstallDir -Recurse -File -Filter 'llama-server.exe' | Select-Object -First 1
if (-not $Server) { throw 'llama-server.exe missing from verified runtime archive' }
Copy-Item -LiteralPath $Server.FullName -Destination (Join-Path $RuntimeRoot 'aetherai-gguf-runtime.exe') -Force
Get-ChildItem -LiteralPath $InstallDir -Recurse -File -Filter '*.dll' | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination (Join-Path $RuntimeRoot $_.Name) -Force }
Get-Verified $ModelUrl (Join-Path $ModelRoot $ModelFile) $ModelSha
Write-Output 'AETHERAI_RUNTIME_ASSET_SHA256=PASS'
Write-Output 'AETHERAI_MODEL_ASSET_SHA256=PASS'
Write-Output "AETHERAI_GGUF_RUNTIME=$(Join-Path $RuntimeRoot 'aetherai-gguf-runtime.exe')"
Write-Output "AETHERAI_STARTER_MODEL=$(Join-Path $ModelRoot $ModelFile)"
Write-Output 'AETHERAI_LIVE_ASSETS=PASS'

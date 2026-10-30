param([string]$BuildDir = 'target/release', [string]$OutputDir = 'dist')
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$build = Join-Path $root $BuildDir
$out = Join-Path $root $OutputDir
$package = Join-Path $out 'aperture-windows'
foreach ($exe in @('aperture-display.exe','aperture.exe')) {
    if (-not (Test-Path -LiteralPath (Join-Path $build $exe))) { throw "Missing build output: $exe" }
}
if (Test-Path -LiteralPath $package) { throw 'Package folder already exists. Choose a fresh OutputDir.' }
New-Item -ItemType Directory -Path $package -Force | Out-Null
Get-ChildItem -LiteralPath (Join-Path $root 'windows') -Force | Copy-Item -Destination $package -Recurse
foreach ($exe in @('aperture-display.exe','aperture.exe')) { Copy-Item -LiteralPath (Join-Path $build $exe) -Destination $package }
foreach ($name in @('LICENSE','NOTICE.md')) { Copy-Item -LiteralPath (Join-Path $root $name) -Destination $package }
New-Item -ItemType Directory -Path (Join-Path $package 'docs/reference'),(Join-Path $package 'assets/brand') -Force | Out-Null
foreach ($name in @('CONFIGURATION.md','MIGRATION.md','VALIDATION.md')) { Copy-Item -LiteralPath (Join-Path $root "docs/$name") -Destination (Join-Path $package 'docs') }
Copy-Item -LiteralPath (Join-Path $root 'docs/reference/OPTIONS_GUIDE.md') -Destination (Join-Path $package 'docs/reference')
foreach ($name in @('OFL.txt','Inter-OFL.txt')) { Copy-Item -LiteralPath (Join-Path $root "assets/$name") -Destination (Join-Path $package 'assets') }
Copy-Item -LiteralPath (Join-Path $root 'assets/brand/aperture.ico') -Destination (Join-Path $package 'assets/brand')
Compress-Archive -Path (Join-Path $package '*') -DestinationPath (Join-Path $out 'aperture-windows.zip')
$hash = (Get-FileHash -LiteralPath (Join-Path $out 'aperture-windows.zip') -Algorithm SHA256).Hash.ToLower()
"$hash  aperture-windows.zip" | Set-Content -LiteralPath (Join-Path $out 'SHA256SUMS.txt') -Encoding ascii
Write-Output "Package ready: $hash"

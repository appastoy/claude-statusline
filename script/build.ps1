#Requires -Version 7
$ErrorActionPreference = 'Stop'

# Tests gate the build
& "$PSScriptRoot\test.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Write-Host "==> Building release binary..."
Push-Location $repoRoot
try {
    cargo build --release
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($exitCode -ne 0) {
    Write-Error "Build failed (exit $exitCode)"
    exit $exitCode
}

$src  = Join-Path $repoRoot 'target\release\statusline.exe'
$dist = Join-Path $repoRoot 'dist'
$dst  = Join-Path $dist 'statusline.exe'

New-Item -ItemType Directory -Force $dist | Out-Null
Copy-Item $src $dst -Force

$size = (Get-Item $dst).Length
Write-Host "==> Built: $dst  ($([math]::Round($size/1KB, 1)) KB)"

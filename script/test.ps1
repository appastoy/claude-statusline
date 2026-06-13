#Requires -Version 7
$ErrorActionPreference = 'Stop'

# Ensure Rust is available
& "$PSScriptRoot\setup.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

$repoRoot = Resolve-Path "$PSScriptRoot\.."
Write-Host "==> Running cargo test..."
Push-Location $repoRoot
try {
    cargo test
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($exitCode -ne 0) {
    Write-Error "Tests failed (exit $exitCode)"
}
exit $exitCode

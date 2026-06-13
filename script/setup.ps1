#Requires -Version 7
$ErrorActionPreference = 'Stop'

function Test-Rust {
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    $rustc = Get-Command rustc -ErrorAction SilentlyContinue
    if (-not $cargo -or -not $rustc) { return $false }
    $null = cargo --version 2>$null
    return $LASTEXITCODE -eq 0
}

Write-Host "==> Checking Rust toolchain..."

# 1. Already on PATH?
if (Test-Rust) {
    Write-Host "  cargo $(cargo --version)  rustc $(rustc --version)"
    Write-Host "==> Rust OK"
    exit 0
}

# 2. Installed but not on PATH this session?
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
    if (Test-Rust) {
        Write-Host "  Added $cargoBin to PATH"
        Write-Host "  cargo $(cargo --version)  rustc $(rustc --version)"
        Write-Host "==> Rust OK"
        exit 0
    }
}

# 3. Download and run rustup-init
Write-Host "  Rust not found. Downloading rustup-init..."
$rustupUrl  = 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
$rustupExe  = Join-Path $env:TEMP 'rustup-init.exe'
Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupExe -UseBasicParsing
Write-Host "  Running rustup-init (minimal stable toolchain)..."
& $rustupExe -y --default-toolchain stable --profile minimal
if ($LASTEXITCODE -ne 0) {
    Write-Error "rustup-init failed with exit code $LASTEXITCODE"
    exit 1
}

# Re-add cargo bin and verify
$env:PATH = "$cargoBin;$env:PATH"
if (-not (Test-Rust)) {
    Write-Error "Rust installed but still not usable. Open a new terminal and retry."
    exit 1
}

Write-Host "  cargo $(cargo --version)  rustc $(rustc --version)"
Write-Host ""
Write-Host "  NOTE: If 'cargo build' later fails with 'link.exe not found',"
Write-Host "  you need the Visual C++ Build Tools:"
Write-Host "  https://visualstudio.microsoft.com/visual-cpp-build-tools/"
Write-Host ""
Write-Host "==> Rust OK"

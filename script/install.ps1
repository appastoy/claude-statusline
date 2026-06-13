#Requires -Version 7
$ErrorActionPreference = 'Stop'

$repoRoot  = Resolve-Path "$PSScriptRoot\.."
$distExe   = Join-Path $repoRoot 'dist\statusline.exe'
$installDir = Join-Path $env:USERPROFILE '.claude\statusline\appastoy'
$targetExe  = Join-Path $installDir 'statusline.exe'
$settingsPath = Join-Path $env:USERPROFILE '.claude\settings.json'

# ── 1. Check dist exe ──────────────────────────────────────────────────────────
if (-not (Test-Path $distExe)) {
    Write-Host ""
    Write-Host "  ERROR: dist\statusline.exe not found." -ForegroundColor Red
    Write-Host ""
    Write-Host "  Build it first:"
    Write-Host "    build.bat"
    Write-Host "  or:"
    Write-Host "    pwsh -File script\build.ps1"
    Write-Host ""
    exit 1
}

# ── 2. Copy exe to install dir ─────────────────────────────────────────────────
Write-Host "==> Installing to $targetExe ..."
New-Item -ItemType Directory -Force $installDir | Out-Null
Copy-Item $distExe $targetExe -Force
$size = (Get-Item $targetExe).Length
Write-Host "  Copied ($([math]::Round($size/1KB, 1)) KB)"

# ── 3. Check / update settings.json ───────────────────────────────────────────
# Claude Code on Windows runs the statusLine command through Git Bash, which
# strips unquoted backslashes ("C:\Users\..." becomes "C:Users..."), so the
# command silently fails with no output. Forward slashes work in every shell.
$expectedCommand = $targetExe -replace '\\', '/'

Write-Host "==> Checking $settingsPath ..."

$settings = if (Test-Path $settingsPath) {
    Get-Content $settingsPath -Raw | ConvertFrom-Json
} else {
    [pscustomobject]@{}
}

$current = $settings.statusLine.command
if ($current -eq $expectedCommand) {
    Write-Host "  statusLine.command already correct — no change needed."
} else {
    if ($current) {
        Write-Host "  Replacing: $current"
    } else {
        Write-Host "  No statusLine.command found — adding."
    }

    # Build the statusLine block
    $statusLine = [pscustomobject]@{
        type    = 'command'
        command = $expectedCommand
    }

    # Add or replace the statusLine property
    if ($settings.PSObject.Properties['statusLine']) {
        $settings.statusLine = $statusLine
    } else {
        $settings | Add-Member -NotePropertyName 'statusLine' -NotePropertyValue $statusLine -Force
    }

    $json = $settings | ConvertTo-Json -Depth 10
    [System.IO.File]::WriteAllText($settingsPath, $json, [System.Text.Encoding]::UTF8)
    Write-Host "  Saved."
    Write-Host "  statusLine.command = $expectedCommand"
}

Write-Host ""
Write-Host "==> Install complete. Restart Claude Code to apply." -ForegroundColor Green

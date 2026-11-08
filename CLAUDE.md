# CLAUDE.md — Agent Guide: claude-statusline

## Project purpose

This project implements a **custom Claude Code statusline in Rust**. The statusline binary
(`statusline.exe`) is invoked by Claude Code on every prompt render and prints 3 lines of
ANSI-colored status information (model/context/time · rate-limits · folder/git) to stdout.

Customizations are made by editing the Rust source in `src/`, then building and installing.
Do **not** create separate PowerShell or shell scripts to implement statusline logic — all
behavior lives in the Rust code.

## Repository structure

```
Cargo.toml              Rust project (edition 2021, serde + serde_json + chrono)
statusline-example.json Fixture — sample Claude Code stdin payload
statusline-mockup.txt   Layout/color spec reference
statusline-input-schema.md  Full stdin JSON schema (all fields, versions, what we parse)
src/
  main.rs               Orchestration: parse stdin → render 3 lines → stdout
  input.rs              Serde structs for the Claude Code JSON payload
  style.rs              ANSI colors, bar chars, fmt_tok, fmt_cost, effort map
  git.rs                git status --porcelain=v2 --branch parser
tests/
  cli.rs                Integration test: pipe JSON into the binary
script/
  setup.ps1             Verify/install Rust toolchain
  test.ps1              setup → cargo test
  build.ps1             test → cargo build --release → copy to dist/
  install.ps1           Copy dist\statusline.exe → ~/.claude/statusline/appastoy/ and wire settings.json
build.bat               Build entry point: calls script/build.ps1
install.bat             Install entry point: calls script/install.ps1
dist/
  statusline.exe        Build output (not committed)
```

## Customization

**All changes to statusline behavior must go through the Rust source.**

- Edit files in `src/` to change what is displayed or how it looks.
- Run `build.bat` to compile and run tests.
- Run `install.bat` to deploy the new binary to Claude Code.
- Restart Claude Code to apply.

Do **not** create a separate `.ps1`, `.sh`, or any other script to replace or wrap the
statusline logic. The Rust binary is the only implementation.

| File | Responsibility |
|------|---------------|
| `src/main.rs`  | Output layout: which segments appear on which line, join order |
| `src/style.rs` | Colors, bar chars, number formatting (`fmt_tok`, `fmt_cost`) |
| `src/git.rs`   | Git branch/status parsing |
| `src/input.rs` | JSON payload schema — add fields here when Claude Code adds new data |

## Build & Test

### One-command build (from repo root)

```bat
build.bat
```

Or with PowerShell directly:

```powershell
pwsh -File script\build.ps1
```

### Step-by-step scripts

| Script | What it does |
|--------|-------------|
| `script\setup.ps1`   | Checks for `cargo`/`rustc`; installs via rustup if missing |
| `script\test.ps1`    | Runs setup, then `cargo test` (unit + integration tests) |
| `script\build.ps1`   | Runs tests, then `cargo build --release`, copies exe to `dist\` |
| `script\install.ps1` | Copies `dist\statusline.exe` to `~/.claude/statusline/appastoy/` and updates `settings.json` |

**Tests gate the build.** `build.ps1` calls `test.ps1` which calls `setup.ps1`.
If tests fail, the build is aborted.

### Verify the build

```powershell
Get-Content statusline-example.json -Raw | dist\statusline.exe
```

Expected: 3 lines of ANSI-colored text (model/ctx/time · limits · folder/git).

## Install

### One-command install (from repo root)

```bat
install.bat
```

Or with PowerShell directly:

```powershell
pwsh -File script\install.ps1
```

### What install does

1. Checks that `dist\statusline.exe` exists — if not, prints an error and tells the user to run `build.bat` first.
2. Creates `~/.claude/statusline/appastoy/` and copies `dist\statusline.exe` there.
3. Reads `~/.claude/settings.json`, checks whether `statusLine.command` already points at the installed exe.
   - If correct: no change.
   - If missing or different: updates the value and saves the file.
4. Prints a success message and reminds the user to restart Claude Code.

### Installed path

```
%USERPROFILE%\.claude\statusline\appastoy\statusline.exe
```

### Full workflow (build + install)

```bat
build.bat
install.bat
```

Then restart Claude Code.

### Agent instructions

If the user says "install the statusline", "set up the statusline", or "apply changes":
1. Check whether `dist\statusline.exe` exists in the repo.
2. If not, run `build.bat` first (runs setup → test → build).
3. Run `install.bat` (or `pwsh -File script\install.ps1`).
4. Report the installed path and confirm `settings.json` was updated.
5. Remind the user to restart Claude Code.

If the user asks to modify the statusline:
1. Edit the appropriate file(s) in `src/`.
2. Run `build.bat` to build (tests run automatically).
3. Run `install.bat` to deploy.
4. Do NOT create any separate script file for the change.

**Always run `install.bat` automatically after every modification, without waiting for the user to ask.**

## Output format

- **Line 1 (top):** model name · effort level · context bar + % + token count (`cur/max`) · session elapsed time · prompt cache (`🔥 Nm` warm countdown / `🧊 cold`; omitted when `prompt_cache` is absent or `caching_observed` is not true)
- **Line 2 (middle):** 5-hour rate limit (remaining %) · 7-day rate limit (remaining %)
- **Line 3 (bottom):** current folder · git branch + status · ahead/behind

Folder rendering: home dir → `~`; a sub-directory of `project_dir` → `<project>\<relative>` (relative part in cyan); otherwise the current folder's leaf name.

Rate-limit bars show **remaining %** (`100 - used_percentage`), as horizontal 10-cell bars that drain and warm in color as the limit runs out (`limit_color`: green ≥50, yellow ≥25, orange ≥10, else red). The 7-day reset shows the Korean weekday: `2d(수) 07:00` / `내일(목) 12:34` / `오늘(금) 23:42`.

Pace marker: when usage runs ahead of the elapsed share of the window by 10+ points (and at least 10% of the window has elapsed), an orange `▼과속` follows the percent, before the reset time. Windows: 5h = 18000s, 7d = 604800s; `over_pace` in `src/main.rs`. Never shown without `resets_at`.

Prompt cache: `🔥 Nm` (green, N = ceil(remaining seconds / 60)) while `warm` and `expires_at` is in the future, else `🧊 cold` (sky). Uses the single-codepoint 🧊 (the snowflake variant with a variation selector misrenders in width calculation). The countdown needs `statusLine.refreshInterval` = 30, which `install.ps1` sets in `settings.json`.

Empty lines are omitted (e.g. no rate-limit data → line 2 is dropped). Lines are separated by `\n`; no trailing newline. Output is raw UTF-8 bytes written directly to stdout.

## Dependencies

- Rust stable (1.70+)
- `serde` + `serde_json` — JSON payload parsing
- `chrono` — local timezone for rate limit reset time formatting
- `git` (optional) — git status info; gracefully omitted when not available

# Claude Code statusLine — stdin JSON schema (reference)

Claude Code pipes a JSON payload to the statusLine command on **stdin** every render.
This file documents the **full current schema**, cross-checked two ways:

- **Empirically captured** from a live session on **Claude Code v2.1.206** (2026-07-11).
- **Official docs:** <https://code.claude.com/docs/en/statusline.md> (doc header last updated 2026-07-10).

The Rust structs that consume this payload live in [`src/input.rs`](src/input.rs). The
**"Parsed"** column below marks which fields we currently deserialize. Serde ignores any
field not declared, so unlisted/new fields are safe to leave out until we want them.

> ⚠️ **Empirical beats docs.** During this check the docs claimed `fast_mode` was removed,
> but the live 2.1.206 payload still includes it. Always re-capture (below) before trusting
> a doc claim about presence.

## How to re-capture the real payload

The binary already dumps raw stdin to a file when `CLAUDE_STATUSLINE_CAPTURE` is set
(see `main.rs`). Claude Code runs the statusLine command through **Git Bash**, so prefix
the command in `~/.claude/settings.json` with the env var (use **forward slashes** — backslash
paths fail silently):

```jsonc
"statusLine": {
  "type": "command",
  "command": "CLAUDE_STATUSLINE_CAPTURE='C:/path/to/capture.json' C:/Users/<you>/.claude/statusline/appastoy/statusline.exe"
}
```

The change hot-reloads — the next render writes `capture.json`. **Revert the command afterward**
and delete the capture (it contains the real `session_id` / `transcript_path`).

## Full schema

Legend — **Presence:** ● always · ○ conditional (only when a condition holds) · ◐ nullable
(key present, value may be `null`). **Parsed:** ✅ deserialized in `input.rs` · — not used.

### Top level

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `session_id` | string | ● | — | Current session UUID |
| `transcript_path` | string | ● | — | Path to the session `.jsonl` transcript |
| `cwd` | string | ● | ✅ | Current working directory (fallback for folder) |
| `prompt_id` | string | ○ | — | **New (v2.1.196+)** — UUID of the prompt being processed; correlates with OTel events |
| `session_name` | string | ○ | — | Only when named via `--name` / `/rename` |
| `version` | string | ● | — | Claude Code version (e.g. `2.1.206`) |
| `exceeds_200k_tokens` | boolean | ● | — | Context has passed 200k tokens |
| `fast_mode` | boolean | ● | — | Fast mode on/off (still present in 2.1.206 despite doc omission) |

### `model`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `model.id` | string | ● | — | Model id, e.g. `claude-opus-4-8[1m]` |
| `model.display_name` | string | ● | ✅ | Human name, e.g. `Opus 4.8 (1M context)` |

### `effort` / `thinking` / `output_style`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `effort.level` | string | ○ | ✅ | `low`\|`medium`\|`high`\|`xhigh`\|`max`; only when the model supports it |
| `thinking.enabled` | boolean | ● | — | Extended thinking toggle |
| `output_style.name` | string | ● | — | Active output style (`default`, …) |

### `workspace`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `workspace.current_dir` | string | ● | ✅ | Directory the session is focused on |
| `workspace.project_dir` | string | ● | ✅ | Project root (folder rel-path is computed against this) |
| `workspace.added_dirs` | array | ● | — | Extra `--add-dir` directories |
| `workspace.repo` | object | ○ | — | **New** — git remote info; only inside a repo with an `origin` remote |
| `workspace.repo.host` | string | ○ | — | e.g. `github.com` |
| `workspace.repo.owner` | string | ○ | — | e.g. `appastoy` |
| `workspace.repo.name` | string | ○ | — | e.g. `claude-statusline` |
| `workspace.git_worktree` | string | ○ | — | Linked-worktree name when inside one |

### `context_window`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `context_window.context_window_size` | number | ● | ✅ | Max context (e.g. `1000000`) → shown as `/max` |
| `context_window.total_input_tokens` | number | ● | ✅ | **Current** context input tokens (since v2.1.132 this is current, not cumulative) → shown as `cur` |
| `context_window.total_output_tokens` | number | ● | — | Current output tokens |
| `context_window.used_percentage` | number | ◐ | ✅ | % of context used → bar + % |
| `context_window.remaining_percentage` | number | ◐ | — | % remaining |
| `context_window.current_usage` | object | ◐ | ✅ | `null` before first API call and just after `/compact` |
| `context_window.current_usage.input_tokens` | number | ● | ✅ | Fallback for `cur` |
| `context_window.current_usage.output_tokens` | number | ● | — | |
| `context_window.current_usage.cache_creation_input_tokens` | number | ● | ✅ | Fallback for `cur` |
| `context_window.current_usage.cache_read_input_tokens` | number | ● | ✅ | Fallback for `cur` |

### `cost`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `cost.total_cost_usd` | number | ● | — | Session cost (we removed cost from the display) |
| `cost.total_duration_ms` | number | ● | ✅ | Wall-clock session time → `☕` segment |
| `cost.total_api_duration_ms` | number | ● | — | Time spent in API calls |
| `cost.total_lines_added` | number | ● | — | Session lines added (candidate `+N`) |
| `cost.total_lines_removed` | number | ● | — | Session lines removed (candidate `-N`) |

### `rate_limits`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `rate_limits` | object | ○ | ✅ | Claude.ai Pro/Max only, after first API response |
| `rate_limits.five_hour.used_percentage` | number | ● | ✅ | 5h window used % → shown as remaining (`100 - used`); bar drains and warms as it runs out |
| `rate_limits.five_hour.resets_at` | number | ● | ✅ | Unix epoch seconds → reset time |
| `rate_limits.seven_day.used_percentage` | number | ● | ✅ | 7d window used % → shown as remaining (`100 - used`) |
| `rate_limits.seven_day.resets_at` | number | ● | ✅ | Unix epoch seconds → reset time (with Korean weekday) |

### `prompt_cache`

| Field | Type | Presence | Parsed | Meaning |
|-------|------|:-:|:-:|---------|
| `prompt_cache` | object | ○ | ✅ | Absent → no cache segment |
| `prompt_cache.warm` | boolean | ○ | ✅ | Cache currently warm |
| `prompt_cache.caching_observed` | boolean | ○ | ✅ | Must be `true` or the segment is hidden |
| `prompt_cache.expires_at` | number\|null | ○ | ✅ | Unix epoch seconds; `🔥 Nm` countdown when warm and in the future, else `🧊 cold` |

### `statusLine` setting: `refreshInterval`

`statusLine.refreshInterval` (seconds) in `settings.json` re-runs the command on a timer in addition to event-driven renders. `install.ps1` sets it to `30` so the cache countdown ticks while idle.

### Conditional objects (not seen in the 2.1.206 capture — need the right context)

| Field | Type | Appears when | Notes |
|-------|------|--------------|-------|
| `pr` | object | An open PR exists for the current branch | `pr.number`, `pr.url`, `pr.review_state` (`approved`\|`pending`\|`changes_requested`\|`draft`) |
| `vim` | object | Vim mode enabled | `vim.mode` = `NORMAL`\|`INSERT`\|`VISUAL`\|`VISUAL LINE` |
| `agent` | object | Running with `--agent` | `agent.name` |
| `worktree` | object | `--worktree` session | `name`, `path`, `branch?`, `original_cwd`, `original_branch?` |

## Fields new since the old fixture (v2.1.173 → v2.1.206)

- `prompt_id` (v2.1.196+)
- `workspace.repo` `{host, owner, name}`

## Version-gated notes

- **v2.1.132** — `context_window.total_input_tokens` / `total_output_tokens` changed from
  cumulative session totals to **current** context usage. Our `cur/max` display relies on this.
- **v2.1.153** — `COLUMNS` / `LINES` env vars exposed to the statusLine process (terminal sizing).
- **v2.1.196** — `prompt_id` added.

## Good candidates to surface next

- `pr.{number, review_state}` — PR status next to the git segment.
- `cost.total_lines_added` / `total_lines_removed` — session diff `+N -N`.
- `workspace.repo.{owner, name}` — `owner/name` label.
- `vim.mode` / `agent.name` — conditional indicators.

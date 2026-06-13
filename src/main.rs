mod input;
mod style;
mod git;

use std::io::{self, Read, Write};
use chrono::{Local, TimeZone};
use input::Root;
use style::*;

fn main() {
    let mut raw = String::new();
    if io::stdin().read_to_string(&mut raw).is_err() { return; }
    if let Ok(path) = std::env::var("CLAUDE_STATUSLINE_CAPTURE") {
        let _ = std::fs::write(&path, &raw);
    }
    // serde_json rejects a leading UTF-8 BOM, which some Windows tools prepend
    let i: Root = match serde_json::from_str(raw.trim_start_matches('\u{feff}')) {
        Ok(v) => v,
        Err(_) => return,
    };

    let separator = sep();

    let mut seg1: Vec<String> = Vec::new();
    let mut seg2: Vec<String> = Vec::new();
    let mut seg3: Vec<String> = Vec::new();

    // ---- Model segment (seg2) ----
    let raw_name = i.model.as_ref().and_then(|m| m.display_name.as_deref()).unwrap_or("");
    // Strip trailing "(…)" e.g. " (1M context)"
    let model = {
        let s = raw_name.trim();
        if let Some(pos) = s.rfind('(') {
            s[..pos].trim().to_string()
        } else {
            s.to_string()
        }
    };
    let cw_max = i.context_window.as_ref().and_then(|c| c.context_window_size).unwrap_or(0.0);

    if !model.is_empty() {
        let mut mtxt = format!("🤖 {}", paint(&model, ORANGE));
        if let Some(label) = fmt_ctx_label(cw_max) {
            mtxt.push_str(&format!("{}", paint(&format!(" {}", label), MAGENTA)));
        }
        if let Some(lvl) = i.effort.as_ref().and_then(|e| e.level.as_deref()) {
            if let Some((short, color)) = effort_short(lvl) {
                mtxt.push(' ');
                mtxt.push_str(&paint(short, color));
            }
        }
        seg2.push(mtxt);
    }

    // ---- Folder segment (seg1) ----
    let cur = i.workspace.as_ref().and_then(|w| w.current_dir.as_deref())
        .or_else(|| i.cwd.as_deref())
        .unwrap_or("")
        .trim_end_matches(['\\', '/']);

    if !cur.is_empty() {
        let folder = folder_label(cur);
        seg1.push(paint(&format!("📂 {}", folder), BLUE));
    }

    // ---- Context window segment (seg2) ----
    let cw = i.context_window.as_ref();
    let ctx_used = cw.and_then(|c| c.used_percentage).unwrap_or(0.0);
    let cw_cur = cw.and_then(|c| c.total_input_tokens)
        .or_else(|| {
            cw.and_then(|c| c.current_usage.as_ref()).map(|u| {
                u.input_tokens.unwrap_or(0.0)
                    + u.cache_creation_input_tokens.unwrap_or(0.0)
                    + u.cache_read_input_tokens.unwrap_or(0.0)
            })
        })
        .unwrap_or(0.0);

    let ctx_color = ctx_color(ctx_used);
    // Cache hit rate
    let hit_txt = if let Some(cu) = cw.and_then(|c| c.current_usage.as_ref()) {
        let cache_read = cu.cache_read_input_tokens.unwrap_or(0.0);
        if cache_read > 0.0 {
            let c_total = cu.input_tokens.unwrap_or(0.0)
                + cu.cache_creation_input_tokens.unwrap_or(0.0)
                + cache_read;
            let hit_pct = (cache_read / c_total * 100.0) as u32;
            let hit_color = if hit_pct >= 75 { GREEN } else if hit_pct >= 40 { YELLOW } else { MUTED };
            format!(" {}", paint(&format!("({}%)", hit_pct), hit_color))
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    seg2.push(format!(
        "🧠 {} {}{}",
        paint(&format!("{} {}%", bar_str(ctx_used), ctx_used as u32), ctx_color),
        paint(&fmt_tok(cw_cur), WHITE),
        hit_txt
    ));

    // ---- Rate limits (seg3) ----
    if let Some(rl) = i.rate_limits.as_ref() {
        if let Some(s) = fmt_limit("⏳ 5h", rl.five_hour.as_ref(), "5h") { seg3.push(s); }
        if let Some(s) = fmt_limit("📅 1w", rl.seven_day.as_ref(), "1w") { seg3.push(s); }
    }

    // ---- Session time (seg3) ----
    let dur_ms = i.cost.as_ref().and_then(|c| c.total_duration_ms).unwrap_or(0.0);
    if dur_ms > 0.0 {
        let min = (dur_ms / 60_000.0).floor() as u64;
        seg3.push(format!("☕ {}", paint(&format!("{}h {}m", min / 60, min % 60), PEACH)));
    }

    // ---- Git (seg1) ----
    if !cur.is_empty() {
        match git::query(cur) {
            Some(gs) => {
                let mut git_txt = format!("🌿 {}", paint(&gs.branch, PURPLE));
                if gs.staged == 0 && gs.modified == 0 && gs.untracked == 0 {
                    git_txt.push(' ');
                    git_txt.push_str(&paint("✓", GREEN));
                } else {
                    let mut marks: Vec<String> = Vec::new();
                    if gs.staged > 0    { marks.push(paint(&format!("✚{}", gs.staged), CYAN)); }
                    if gs.modified > 0  { marks.push(paint(&format!("●{}", gs.modified), YELLOW)); }
                    if gs.untracked > 0 { marks.push(paint(&format!("?{}", gs.untracked), MUTED)); }
                    git_txt.push(' ');
                    git_txt.push_str(&marks.join(" "));
                }
                seg1.push(git_txt);

                if gs.ahead > 0 || gs.behind > 0 {
                    let mut ab: Vec<String> = Vec::new();
                    if gs.ahead > 0  { ab.push(paint(&format!("↑{}", gs.ahead), SKY)); }
                    if gs.behind > 0 { ab.push(paint(&format!("↓{}", gs.behind), ORANGE)); }
                    seg1.push(ab.join(" "));
                }
            }
            None => {
                seg1.push(paint("🌿 no git", MUTED));
            }
        }
    }

    // ---- Cost (seg1) ----
    if let Some(cost) = i.cost.as_ref().and_then(|c| c.total_cost_usd) {
        seg1.push(paint(&format!("💰 {}", fmt_cost(cost)), GOLD));
    }

    // ---- Emit output ----
    let line1 = seg1.join(&separator);
    let line2 = seg2.join(&separator);
    let line3 = seg3.join(&separator);

    let output = if !line3.is_empty() {
        format!("{}\n{}\n{}", line2, line3, line1)
    } else if !line2.is_empty() {
        format!("{}\n{}", line2, line1)
    } else {
        line1
    };

    let stdout = io::stdout();
    let _ = stdout.lock().write_all(output.as_bytes());
}

fn folder_label(cur: &str) -> String {
    let parts: Vec<&str> = cur.split(['\\', '/']).filter(|s| !s.is_empty()).collect();
    if parts.len() >= 2 {
        let folder = format!("{}\\{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        // Drive-root fix: "D:" → "D:\"
        if folder.len() == 2 && folder.ends_with(':') {
            format!("{}\\", folder)
        } else {
            folder
        }
    } else if parts.len() == 1 {
        let s = parts[0].to_string();
        if s.len() == 2 && s.ends_with(':') { format!("{}\\", s) } else { s }
    } else {
        cur.to_string()
    }
}

fn fmt_limit(label: &str, node: Option<&input::RateLimit>, kind: &str) -> Option<String> {
    let node = node?;
    let used_pct = node.used_percentage?;
    let rem = 100.0 - used_pct;
    let rem_i = rem.round() as i64;
    let lc = limit_color(rem);
    let mut result = format!("{} {}", label, paint(&format!("{} {}%", bar_str(rem), rem_i), lc));

    if let Some(resets_at) = node.resets_at {
        let reset_local = Local.timestamp_opt(resets_at as i64, 0).single();
        if let Some(reset) = reset_local {
            if kind == "5h" {
                result.push_str(&paint(&format!(" {}", reset.format("%H:%M")), MUTED));
            } else {
                let today = Local::now().date_naive();
                let reset_date = reset.date_naive();
                let days = (reset_date - today).num_days();
                result.push_str(&paint(&format!(" {}d {}", days, reset.format("%H:%M")), MUTED));
            }
        }
    }

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_label_two_depth() {
        assert_eq!(folder_label("D:\\github\\myproject"), "github\\myproject");
        assert_eq!(folder_label("D:\\github"), "D:\\github");
        assert_eq!(folder_label("/home/user/project"), "user\\project");
    }

    #[test]
    fn folder_label_drive_root() {
        // single segment that is a drive letter
        assert_eq!(folder_label("D:"), "D:\\");
    }
}

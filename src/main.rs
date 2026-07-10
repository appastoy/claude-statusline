mod input;
mod style;
mod git;

use std::io::{self, Read, Write};
use chrono::{Datelike, Local, TimeZone};
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

    // ---- Model segment (seg2 → line 1) ----
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
        if let Some(lvl) = i.effort.as_ref().and_then(|e| e.level.as_deref()) {
            if let Some((short, color)) = effort_short(lvl) {
                mtxt.push(' ');
                mtxt.push_str(&paint(short, color));
            }
        }
        seg2.push(mtxt);
    }

    // ---- Folder segment (seg1 → line 3) ----
    let cur = i.workspace.as_ref().and_then(|w| w.current_dir.as_deref())
        .or_else(|| i.cwd.as_deref())
        .unwrap_or("")
        .trim_end_matches(['\\', '/']);

    if !cur.is_empty() {
        let project = i.workspace.as_ref().and_then(|w| w.project_dir.as_deref());
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok();
        seg1.push(folder_segment(cur, project, home.as_deref()));
    }

    // ---- Context window segment (seg2 → line 1) ----
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
    // Token count as current/max: current bright (WHITE), max toned down (DIM); e.g. "47k/1M"
    let tok_txt = if cw_max > 0.0 {
        format!(
            "{}{}",
            paint(&fmt_tok(cw_cur), WHITE),
            paint(&format!("/{}", fmt_tok(cw_max)), DIM),
        )
    } else {
        paint(&fmt_tok(cw_cur), WHITE)
    };

    seg2.push(format!(
        "🧠 {} {}",
        paint(&format!("{} {}%", bar_str(ctx_used), ctx_used as u32), ctx_color),
        tok_txt,
    ));

    // ---- Session time (seg2 → line 1, last) ----
    let dur_ms = i.cost.as_ref().and_then(|c| c.total_duration_ms).unwrap_or(0.0);
    if dur_ms > 0.0 {
        let min = (dur_ms / 60_000.0).floor() as u64;
        seg2.push(format!("☕ {}", paint(&format!("{}h {}m", min / 60, min % 60), PEACH)));
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

    // ---- Rate limits (seg3 → line 2) ----
    if let Some(rl) = i.rate_limits.as_ref() {
        if let Some(s) = fmt_limit("⏳ 5h", rl.five_hour.as_ref(), "5h") { seg3.push(s); }
        if let Some(s) = fmt_limit("📅 7d", rl.seven_day.as_ref(), "7d") { seg3.push(s); }
    }

    // ---- Emit output ----
    // line 1 = seg2 (model · ctx · time)
    // line 2 = seg3 (5h · 7d)
    // line 3 = seg1 (folder · git · ahead/behind)
    let line1 = seg2.join(&separator);
    let line2 = seg3.join(&separator);
    let line3 = seg1.join(&separator);

    let output = [line1, line2, line3]
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let stdout = io::stdout();
    let _ = stdout.lock().write_all(output.as_bytes());
}

/// Render the folder segment for line 2.
/// - current == home            → "📂 ~"                    (BLUE)
/// - current under project_dir  → "📂 <proj>" + "\<rel>"    (BLUE + CYAN)
/// - otherwise                  → "📂 <leaf>"               (BLUE)
fn folder_segment(cur: &str, project: Option<&str>, home: Option<&str>) -> String {
    if let Some(h) = home {
        if path_eq(cur, h) {
            return paint("📂 ~", BLUE);
        }
    }
    if let Some(p) = project {
        if let Some(rel) = rel_path(cur, p) {
            return format!(
                "{}{}",
                paint(&format!("📂 {}", leaf(p)), BLUE),
                paint(&format!("\\{}", rel), CYAN),
            );
        }
    }
    paint(&format!("📂 {}", leaf(cur)), BLUE)
}

/// Normalize a path for comparison: drop trailing separators, unify to '\', lowercase.
fn norm(p: &str) -> String {
    p.trim_end_matches(['\\', '/']).replace('/', "\\").to_lowercase()
}

fn path_eq(a: &str, b: &str) -> bool {
    norm(a) == norm(b)
}

/// If `cur` is strictly below `project`, return the relative remainder using '\' separators.
fn rel_path(cur: &str, project: &str) -> Option<String> {
    let cur = cur.trim_end_matches(['\\', '/']);
    let p = project.trim_end_matches(['\\', '/']);
    if p.is_empty() {
        return None;
    }
    let cn = norm(cur);
    let pn = norm(p);
    if cn.len() > pn.len() && cn.starts_with(&pn) && cn.as_bytes()[pn.len()] == b'\\' {
        cur.get(pn.len()..)
            .map(|tail| tail.trim_start_matches(['\\', '/']).replace('/', "\\"))
    } else {
        None
    }
}

/// Last path component; drive-root "D:" renders as "D:\".
fn leaf(path: &str) -> String {
    let parts: Vec<&str> = path.split(['\\', '/']).filter(|s| !s.is_empty()).collect();
    match parts.last() {
        Some(s) if s.len() == 2 && s.ends_with(':') => format!("{}\\", s),
        Some(s) => s.to_string(),
        None => path.to_string(),
    }
}

fn fmt_limit(label: &str, node: Option<&input::RateLimit>, kind: &str) -> Option<String> {
    let node = node?;
    let used_pct = node.used_percentage?;
    let used_i = used_pct.round() as i64;
    // Show usage % (matches Claude's web usage page and the raw stdin); warms as it fills
    let lc = ctx_color(used_pct);
    let mut result = format!("{} {}", label, paint(&format!("{} {}%", bar_str(used_pct), used_i), lc));

    if let Some(resets_at) = node.resets_at {
        let reset_local = Local.timestamp_opt(resets_at as i64, 0).single();
        if let Some(reset) = reset_local {
            if kind == "5h" {
                result.push_str(&paint(&format!(" {}", reset.format("%H:%M")), MUTED));
            } else {
                let today = Local::now().date_naive();
                let reset_date = reset.date_naive();
                let days = (reset_date - today).num_days();
                let wd = kor_weekday(reset.weekday());
                let hm = reset.format("%H:%M");
                let txt = if days >= 2 {
                    format!(" {}d({}) {}", days, wd, hm)
                } else if days == 1 {
                    format!(" 내일({}) {}", wd, hm)
                } else {
                    format!(" 오늘({}) {}", wd, hm)
                };
                result.push_str(&paint(&txt, MUTED));
            }
        }
    }

    Some(result)
}

/// Korean single-letter weekday for a reset date.
fn kor_weekday(wd: chrono::Weekday) -> &'static str {
    use chrono::Weekday::*;
    match wd {
        Mon => "월",
        Tue => "화",
        Wed => "수",
        Thu => "목",
        Fri => "금",
        Sat => "토",
        Sun => "일",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c2 in chars.by_ref() {
                    if c2 == 'm' { break; }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn leaf_cases() {
        assert_eq!(leaf("D:\\Workspace\\claude-statusline"), "claude-statusline");
        assert_eq!(leaf("/home/user/project"), "project");
        assert_eq!(leaf("D:\\github"), "github");
        assert_eq!(leaf("D:"), "D:\\");
    }

    #[test]
    fn rel_path_cases() {
        assert_eq!(rel_path("D:\\proj\\src\\ui", "D:\\proj"), Some("src\\ui".to_string()));
        assert_eq!(rel_path("D:/proj/src", "D:\\proj\\"), Some("src".to_string()));
        assert_eq!(rel_path("D:\\proj", "D:\\proj"), None);
        assert_eq!(rel_path("D:\\proj2\\x", "D:\\proj"), None);
        assert_eq!(rel_path("D:\\other", "D:\\proj"), None);
    }

    #[test]
    fn folder_segment_cases() {
        let home = Some("C:\\Users\\me");
        assert_eq!(strip(&folder_segment("C:\\Users\\me", None, home)), "📂 ~");
        assert_eq!(strip(&folder_segment("c:\\users\\ME", None, home)), "📂 ~");
        assert_eq!(strip(&folder_segment("D:\\proj", Some("D:\\proj"), home)), "📂 proj");
        assert_eq!(strip(&folder_segment("D:\\proj\\src\\ui", Some("D:\\proj"), home)), "📂 proj\\src\\ui");
        assert_eq!(strip(&folder_segment("D:\\proj\\src", None, home)), "📂 src");
        assert_eq!(strip(&folder_segment("D:\\other\\x", Some("D:\\proj"), home)), "📂 x");
    }

    #[test]
    fn kor_weekday_maps() {
        use chrono::Weekday;
        assert_eq!(kor_weekday(Weekday::Mon), "월");
        assert_eq!(kor_weekday(Weekday::Fri), "금");
        assert_eq!(kor_weekday(Weekday::Sun), "일");
    }
}

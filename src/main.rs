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

    // ---- Prompt cache (seg2 → line 1, after session time) ----
    if let Some(s) = cache_segment(i.prompt_cache.as_ref(), now_secs()) { seg2.push(s); }

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
        if let Some(s) = fmt_limit("⏳ 5h", rl.five_hour.as_ref(), "5h", now_secs()) { seg3.push(s); }
        if let Some(s) = fmt_limit("📅 7d", rl.seven_day.as_ref(), "7d", now_secs()) { seg3.push(s); }
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

/// Current Unix time in seconds.
fn now_secs() -> f64 {
    Local::now().timestamp() as f64
}

const PACE_MIN_ELAPSED_PCT: f64 = 10.0;
const PACE_MIN_OVERSHOOT_PCT: f64 = 10.0;

/// True when usage runs ahead of the elapsed share of the window by 10+ points.
/// Ignored during the first 10% of the window and when `resets_at` is unknown.
fn over_pace(used_pct: f64, resets_at: Option<f64>, now: f64, window_secs: f64) -> bool {
    let Some(resets_at) = resets_at else { return false };
    if window_secs <= 0.0 { return false; }
    let remaining = resets_at - now;
    let elapsed_pct = ((window_secs - remaining) * 100.0 / window_secs).clamp(0.0, 100.0);
    elapsed_pct >= PACE_MIN_ELAPSED_PCT && used_pct - elapsed_pct >= PACE_MIN_OVERSHOOT_PCT
}

/// Render the prompt cache segment: "🔥 Nm" while warm, "🧊 cold" otherwise.
/// None when the cache state is absent or caching was never observed.
fn cache_segment(pc: Option<&input::PromptCache>, now: f64) -> Option<String> {
    let pc = pc?;
    if pc.caching_observed != Some(true) { return None; }
    if pc.warm == Some(true) {
        if let Some(exp) = pc.expires_at {
            if exp > now {
                let mins = ((exp - now) / 60.0).ceil() as u64;
                return Some(format!("🔥 {}", paint(&format!("{}m", mins), GREEN)));
            }
        }
    }
    Some(format!("🧊 {}", paint("cold", SKY)))
}

fn fmt_limit(label: &str, node: Option<&input::RateLimit>, kind: &str, now: f64) -> Option<String> {
    let node = node?;
    let used_pct = node.used_percentage?;
    // Show remaining %: the bar drains and warms as the limit runs out
    let rem = (100.0 - used_pct).clamp(0.0, 100.0);
    let rem_i = rem.round() as i64;
    let lc = limit_color(rem);
    let mut result = format!("{} {}", label, paint(&format!("{} {}%", bar_str(rem), rem_i), lc));

    let window = if kind == "5h" { 18_000.0 } else { 604_800.0 };
    if over_pace(used_pct, node.resets_at, now, window) {
        result.push_str(&paint(" ▼과속", ORANGE));
    }

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
    fn over_pace_boundaries() {
        let w = 18_000.0;
        let now = 1_000_000.0;
        // 50% elapsed (9000s left)
        let r = Some(now + 9_000.0);
        assert!(over_pace(60.0, r, now, w));   // exactly 10 over
        assert!(!over_pace(59.9, r, now, w));  // just below
        assert!(over_pace(90.0, r, now, w));
        assert!(!over_pace(40.0, r, now, w));  // under pace
        // exactly 10% elapsed (16200s left): 20 used is exactly 10 over
        assert!(over_pace(20.0, Some(now + 16_200.0), now, w));
        // elapsed just under 10% (16201s left): suppressed
        assert!(!over_pace(50.0, Some(now + 16_201.0), now, w));
        assert!(!over_pace(30.0, Some(now + 17_000.0), now, w));
        // reset already past: elapsed clamps to 100, never over pace
        assert!(!over_pace(100.0, Some(now - 5.0), now, w));
        // no resets_at
        assert!(!over_pace(100.0, None, now, w));
        // 7d window
        let wk = 604_800.0;
        assert!(over_pace(60.0, Some(now + wk / 2.0), now, wk));
        assert!(!over_pace(59.0, Some(now + wk / 2.0), now, wk));
    }

    #[test]
    fn fmt_limit_pace_marker() {
        let now = 1_000_000.0;
        let node = input::RateLimit { used_percentage: Some(60.0), resets_at: Some(now + 9_000.0) };
        assert!(strip(&fmt_limit("⏳ 5h", Some(&node), "5h", now).unwrap()).contains("40% ▼과속 "));
        let ok = input::RateLimit { used_percentage: Some(50.0), resets_at: Some(now + 9_000.0) };
        assert!(!strip(&fmt_limit("⏳ 5h", Some(&ok), "5h", now).unwrap()).contains("과속"));
    }

    #[test]
    fn cache_segment_cases() {
        let now = 1_000_000.0;
        let pc = |warm, obs, exp| input::PromptCache { warm, caching_observed: obs, expires_at: exp };
        assert_eq!(cache_segment(None, now), None);
        assert_eq!(cache_segment(Some(&pc(Some(true), None, Some(now + 60.0))), now), None);
        assert_eq!(cache_segment(Some(&pc(Some(true), Some(false), Some(now + 60.0))), now), None);
        let t = |p: input::PromptCache| strip(&cache_segment(Some(&p), now).unwrap());
        assert_eq!(t(pc(Some(true), Some(true), Some(now + 60.0))), "🔥 1m");
        assert_eq!(t(pc(Some(true), Some(true), Some(now + 61.0))), "🔥 2m");
        assert_eq!(t(pc(Some(true), Some(true), Some(now + 1.0))), "🔥 1m");
        assert_eq!(t(pc(Some(true), Some(true), Some(now))), "🧊 cold");
        assert_eq!(t(pc(Some(true), Some(true), None)), "🧊 cold");
        assert_eq!(t(pc(Some(false), Some(true), Some(now + 60.0))), "🧊 cold");
        assert_eq!(t(pc(None, Some(true), Some(now + 60.0))), "🧊 cold");
    }

    #[test]
    fn kor_weekday_maps() {
        use chrono::Weekday;
        assert_eq!(kor_weekday(Weekday::Mon), "월");
        assert_eq!(kor_weekday(Weekday::Fri), "금");
        assert_eq!(kor_weekday(Weekday::Sun), "일");
    }
}

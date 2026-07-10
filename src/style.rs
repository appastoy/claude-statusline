pub const GREEN: &str = "38;5;71";
pub const YELLOW: &str = "38;5;179";
pub const ORANGE: &str = "38;5;208";
pub const RED: &str = "38;5;167";
pub const MUTED: &str = "38;5;240";
pub const CYAN: &str = "38;5;80";
pub const BLUE: &str = "38;5;75";
pub const PURPLE: &str = "38;5;140";
pub const WHITE: &str = "38;5;255";
pub const DIM: &str = "38;5;245";
pub const SKY: &str = "38;5;110";
pub const PEACH: &str = "38;5;215";

pub fn paint(text: &str, code: &str) -> String {
    format!("\x1b[{}m{}\x1b[0m", code, text)
}

pub fn sep() -> String {
    paint(" │ ", MUTED)
}

pub fn ctx_color(used: f64) -> &'static str {
    if used < 50.0 { GREEN }
    else if used < 75.0 { YELLOW }
    else if used < 90.0 { ORANGE }
    else { RED }
}

pub fn bar_str(pct: f64) -> String {
    let filled = ((pct / 100.0 * 10.0).round() as usize).min(10);
    let empty = 10 - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

pub fn fmt_tok(n: f64) -> String {
    if n >= 1_000_000.0 {
        let m = n / 1_000_000.0;
        if m == m.floor() {
            format!("{}M", m as u64)
        } else {
            format!("{:.1}M", m)
        }
    } else {
        format!("{}k", (n / 1000.0).round() as i64)
    }
}

pub fn effort_short(level: &str) -> Option<(&'static str, &'static str)> {
    match level {
        "low"       => Some(("L", "38;5;240")),
        "medium"    => Some(("M", "38;5;109")),
        "high"      => Some(("H", "38;5;71")),
        "xhigh"     => Some(("X", "38;5;179")),
        "max"       => Some(("MX", "38;5;208")),
        _           => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fmt_tok_cases() {
        assert_eq!(fmt_tok(0.0), "0k");
        assert_eq!(fmt_tok(123_000.0), "123k");
        assert_eq!(fmt_tok(200_000.0), "200k");
        assert_eq!(fmt_tok(1_000_000.0), "1M");
        assert_eq!(fmt_tok(1_500_000.0), "1.5M");
        assert_eq!(fmt_tok(47_432.0), "47k");
    }

    #[test]
    fn bar_str_bounds() {
        assert_eq!(bar_str(0.0),   "░░░░░░░░░░");
        assert_eq!(bar_str(100.0), "██████████");
        assert_eq!(bar_str(50.0),  "█████░░░░░");
        assert_eq!(bar_str(10.0),  "█░░░░░░░░░");
        assert_eq!(bar_str(90.0),  "█████████░");
    }

    #[test]
    fn ctx_color_thresholds() {
        assert_eq!(ctx_color(0.0), GREEN);
        assert_eq!(ctx_color(49.9), GREEN);
        assert_eq!(ctx_color(50.0), YELLOW);
        assert_eq!(ctx_color(74.9), YELLOW);
        assert_eq!(ctx_color(75.0), ORANGE);
        assert_eq!(ctx_color(89.9), ORANGE);
        assert_eq!(ctx_color(90.0), RED);
    }

    #[test]
    fn effort_mapping() {
        assert_eq!(effort_short("high"), Some(("H", "38;5;71")));
        assert_eq!(effort_short("max"), Some(("MX", "38;5;208")));
        assert_eq!(effort_short("unknown"), None);
    }
}

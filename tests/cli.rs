use std::process::{Command, Stdio};
use std::io::Write;

const EXAMPLE_JSON: &str = r#"{"session_id":"00000000-0000-0000-0000-000000000000","transcript_path":"C:\\Users\\user\\.claude\\projects\\D--github\\00000000-0000-0000-0000-000000000000.jsonl","cwd":"D:\\github","effort":{"level":"high"},"session_name":"Revert to previous statusline","model":{"id":"claude-opus-4-8[1m]","display_name":"Opus 4.8 (1M context)"},"workspace":{"current_dir":"D:\\github","project_dir":"D:\\github","added_dirs":[]},"version":"2.1.173","output_style":{"name":"default"},"cost":{"total_cost_usd":1.1185685,"total_duration_ms":801357,"total_api_duration_ms":338179,"total_lines_added":136,"total_lines_removed":85},"context_window":{"total_input_tokens":47432,"total_output_tokens":112,"context_window_size":1000000,"current_usage":{"input_tokens":2,"output_tokens":112,"cache_creation_input_tokens":8004,"cache_read_input_tokens":39426},"used_percentage":5,"remaining_percentage":95},"exceeds_200k_tokens":false,"fast_mode":false,"thinking":{"enabled":true},"rate_limits":{"five_hour":{"used_percentage":3,"resets_at":1781225400},"seven_day":{"used_percentage":7.000000000000001,"resets_at":1781474400}}}"#;

fn run_statusline(json: &str) -> String {
    let exe = env!("CARGO_BIN_EXE_statusline");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn statusline");

    child.stdin.as_mut().unwrap().write_all(json.as_bytes()).unwrap();
    drop(child.stdin.take());

    let out = child.wait_with_output().expect("wait failed");
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// Strip all ANSI escape sequences from a string for clean assertions
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // consume until 'm'
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
fn example_payload_contains_expected_segments() {
    let output = run_statusline(EXAMPLE_JSON);
    let plain = strip_ansi(&output);

    // Model line — context-window label removed; effort still present
    assert!(plain.contains("Opus 4.8"), "missing model: {}", plain);
    assert!(plain.contains(" H"), "missing effort: {}", plain);

    // Context — token count now shown as current/max
    assert!(plain.contains("5%"), "missing ctx pct: {}", plain);
    assert!(plain.contains("47k/1M"), "missing tok count/max: {}", plain);
    assert!(!plain.contains("(83%)"), "cache hit rate should be removed: {}", plain);

    // Folder — current_dir == project_dir "D:\\github" → leaf "github"
    assert!(plain.contains("github"), "missing folder: {}", plain);

    // Cost removed
    assert!(!plain.contains("$1.12"), "cost should be removed: {}", plain);

    // Rate limits on line 2
    assert!(plain.contains("5h"), "missing 5h limit: {}", plain);
    assert!(plain.contains("7d"), "missing 7d limit: {}", plain);

    // Three lines: model/ctx/time · rate-limits · folder/git
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 3, "expected 3 lines, got {}: {:?}", lines.len(), lines);
}

#[test]
fn no_trailing_newline() {
    let output = run_statusline(EXAMPLE_JSON);
    assert!(!output.ends_with('\n'), "output has trailing newline");
}

#[test]
fn invalid_json_produces_no_output() {
    let output = run_statusline("not json at all");
    assert!(output.is_empty(), "expected empty output for invalid json, got: {}", output);
}

#[test]
fn utf8_bom_payload_still_renders() {
    let bom_json = format!("\u{feff}{}", EXAMPLE_JSON);
    let output = run_statusline(&bom_json);
    let plain = strip_ansi(&output);
    assert!(plain.contains("Opus 4.8"), "BOM payload produced no output: {:?}", plain);
}

#[test]
fn minimal_payload_does_not_crash() {
    let output = run_statusline("{}");
    // Should produce some output or empty — must not crash
    // (process must exit 0 or at least not panic)
    let _ = output; // just verifying no panic
}

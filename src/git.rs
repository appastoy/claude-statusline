pub struct GitStatus {
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    pub staged: u32,
    pub modified: u32,
    pub untracked: u32,
}

pub fn query(dir: &str) -> Option<GitStatus> {
    let output = std::process::Command::new("git")
        .args(["-C", dir, "--no-optional-locks", "status", "--porcelain=v2", "--branch"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut branch = String::new();
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut staged = 0u32;
    let mut modified = 0u32;
    let mut untracked = 0u32;

    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for tok in rest.split_whitespace() {
                if let Some(n) = tok.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = tok.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with("1 ") || line.starts_with("2 ") {
            let xy: Vec<char> = line.chars().skip(2).take(2).collect();
            if xy.len() == 2 {
                if xy[0] != '.' { staged += 1; }
                if xy[1] != '.' { modified += 1; }
            }
        } else if line.starts_with("? ") {
            untracked += 1;
        }
    }

    if branch.is_empty() {
        return None;
    }

    if branch == "(detached)" {
        branch = "detached".to_string();
    }

    Some(GitStatus { branch, ahead, behind, staged, modified, untracked })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_lines(input: &str) -> GitStatus {
        let mut branch = String::new();
        let mut ahead = 0u32;
        let mut behind = 0u32;
        let mut staged = 0u32;
        let mut modified = 0u32;
        let mut untracked = 0u32;

        for line in input.lines() {
            if let Some(rest) = line.strip_prefix("# branch.head ") {
                branch = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
                for tok in rest.split_whitespace() {
                    if let Some(n) = tok.strip_prefix('+') {
                        ahead = n.parse().unwrap_or(0);
                    } else if let Some(n) = tok.strip_prefix('-') {
                        behind = n.parse().unwrap_or(0);
                    }
                }
            } else if line.starts_with("1 ") || line.starts_with("2 ") {
                let xy: Vec<char> = line.chars().skip(2).take(2).collect();
                if xy.len() == 2 {
                    if xy[0] != '.' { staged += 1; }
                    if xy[1] != '.' { modified += 1; }
                }
            } else if line.starts_with("? ") {
                untracked += 1;
            }
        }

        GitStatus { branch, ahead, behind, staged, modified, untracked }
    }

    #[test]
    fn clean_repo() {
        let input = "# branch.oid abc123\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +0 -0\n";
        let s = parse_lines(input);
        assert_eq!(s.branch, "main");
        assert_eq!(s.ahead, 0);
        assert_eq!(s.behind, 0);
        assert_eq!(s.staged, 0);
        assert_eq!(s.modified, 0);
        assert_eq!(s.untracked, 0);
    }

    #[test]
    fn dirty_repo() {
        let input = "# branch.head feat/login\n# branch.ab +2 -1\n1 M. N... 100644 100644 100644 abc def file.rs\n1 .M N... 100644 100644 100644 abc def file2.rs\n? untracked.txt\n";
        let s = parse_lines(input);
        assert_eq!(s.branch, "feat/login");
        assert_eq!(s.ahead, 2);
        assert_eq!(s.behind, 1);
        assert_eq!(s.staged, 1);
        assert_eq!(s.modified, 1);
        assert_eq!(s.untracked, 1);
    }
}

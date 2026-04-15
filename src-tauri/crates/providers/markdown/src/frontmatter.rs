/// Parsed frontmatter from a Markdown file.
///
/// Frontmatter is an optional YAML block at the very start of the file,
/// delimited by `---` on its own line:
///
/// ```markdown
/// ---
/// title: My Note
/// color: blue
/// tags: [work, ideas]
/// ---
///
/// Note body starts here.
/// ```
#[derive(Debug, Default)]
pub struct Frontmatter {
    pub title:  Option<String>,
    pub color:  Option<String>,
    pub tags:   Vec<String>,
    pub pinned: bool,
}

/// Split file content into (frontmatter, body).
///
/// Returns `(None, full_content)` if no frontmatter block is found.
pub fn parse(content: &str) -> (Frontmatter, &str) {
    let Some(rest) = content.strip_prefix("---") else {
        return (Frontmatter::default(), content);
    };

    // The opening `---` may be immediately followed by a newline or be on its own line.
    let rest = rest.strip_prefix('\n').or_else(|| rest.strip_prefix("\r\n")).unwrap_or(rest);

    // Find the closing `---`
    let end_marker = find_closing_marker(rest);
    let Some((yaml_block, after_marker)) = end_marker else {
        return (Frontmatter::default(), content);
    };

    let fm = parse_yaml(yaml_block);
    (fm, after_marker.trim_start_matches(['\n', '\r']))
}

fn find_closing_marker(rest: &str) -> Option<(&str, &str)> {
    for (i, line) in rest.lines().enumerate() {
        if line.trim() == "---" {
            // Calculate byte offset of start of this line
            let offset = line_start_offset(rest, i)?;
            let body_start = offset + line.len();
            let body_start = body_start
                + rest[body_start..].find(|c| c == '\n' || c == '\r').map_or(0, |p| p + 1);
            return Some((&rest[..offset], &rest[body_start..]));
        }
    }
    None
}

fn line_start_offset(s: &str, target_line: usize) -> Option<usize> {
    if target_line == 0 {
        return Some(0);
    }
    let mut current = 0;
    for (line_idx, line) in s.lines().enumerate() {
        // Advance past this line including its newline
        current += line.len();
        // Account for \r\n vs \n
        if s[current..].starts_with("\r\n") {
            current += 2;
        } else if s[current..].starts_with('\n') {
            current += 1;
        }
        if line_idx + 1 == target_line {
            return Some(current);
        }
    }
    None
}

/// Minimal YAML key-value parser.
///
/// We only need a handful of simple types — no need for a full YAML library:
/// - `key: value` (scalar string)
/// - `key: [a, b, c]` (inline sequence)
fn parse_yaml(block: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();

    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else { continue };
        let key = key.trim();
        let value = value.trim();

        match key {
            "title" => fm.title = Some(unquote(value).to_string()),
            "color" => fm.color = Some(unquote(value).to_lowercase()),
            "pinned" | "pin" => fm.pinned = matches!(value, "true" | "yes" | "1"),
            "tags" | "labels" => {
                fm.tags = parse_inline_sequence(value)
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
            _ => {}
        }
    }

    fm
}

/// Strip surrounding quotes from a YAML scalar value.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"'))
        || (s.starts_with('\'') && s.ends_with('\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Parse a YAML inline sequence: `[a, b, c]` → `["a", "b", "c"]`.
fn parse_inline_sequence(s: &str) -> Vec<&str> {
    let s = s.trim();
    let inner = if s.starts_with('[') && s.ends_with(']') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    inner
        .split(',')
        .map(|item| unquote(item.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_frontmatter_returns_full_content() {
        let (fm, body) = parse("# Hello\n\nSome text.");
        assert!(fm.title.is_none());
        assert_eq!(body, "# Hello\n\nSome text.");
    }

    #[test]
    fn parses_title_and_color() {
        let content = "---\ntitle: My Note\ncolor: blue\n---\n\nBody here.";
        let (fm, body) = parse(content);
        assert_eq!(fm.title.as_deref(), Some("My Note"));
        assert_eq!(fm.color.as_deref(), Some("blue"));
        assert_eq!(body.trim(), "Body here.");
    }

    #[test]
    fn parses_tags_sequence() {
        let content = "---\ntags: [work, ideas, todo]\n---\nBody.";
        let (fm, _) = parse(content);
        assert_eq!(fm.tags, vec!["work", "ideas", "todo"]);
    }

    #[test]
    fn parses_pinned_flag() {
        let content = "---\npinned: true\n---\nBody.";
        let (fm, _) = parse(content);
        assert!(fm.pinned);
    }

    #[test]
    fn quoted_title() {
        let content = "---\ntitle: \"Quoted Title\"\n---\nBody.";
        let (fm, _) = parse(content);
        assert_eq!(fm.title.as_deref(), Some("Quoted Title"));
    }
}

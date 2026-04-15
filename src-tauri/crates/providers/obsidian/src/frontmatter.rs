use chrono::{DateTime, Utc};

/// Parsed frontmatter for an Obsidian note.
///
/// Obsidian uses standard YAML frontmatter (`---` block) with additional
/// conventions: `created`, `updated`, `aliases`, `status`, and inline `#tags`.
#[derive(Debug, Default)]
pub struct ObsidianFrontmatter {
    pub title:   Option<String>,
    pub aliases: Vec<String>,
    pub color:   Option<String>,
    pub tags:    Vec<String>,
    pub pinned:  bool,
    pub created: Option<DateTime<Utc>>,
    pub updated: Option<DateTime<Utc>>,
}

/// Split file content into (frontmatter, body).
/// Returns `(None, full_content)` if no frontmatter block is found.
pub fn parse(content: &str) -> (ObsidianFrontmatter, &str) {
    let Some(rest) = content.strip_prefix("---") else {
        return (ObsidianFrontmatter::default(), content);
    };
    let rest = rest.strip_prefix('\n').or_else(|| rest.strip_prefix("\r\n")).unwrap_or(rest);

    let Some((yaml_block, after)) = find_closing_marker(rest) else {
        return (ObsidianFrontmatter::default(), content);
    };

    let fm = parse_yaml(yaml_block);
    (fm, after.trim_start_matches(['\n', '\r']))
}

fn find_closing_marker(rest: &str) -> Option<(&str, &str)> {
    for (i, line) in rest.lines().enumerate() {
        if line.trim() == "---" {
            let offset = line_start_offset(rest, i)?;
            let body_start = offset + line.len();
            let body_start = body_start
                + rest[body_start..].find(|c| c == '\n' || c == '\r').map_or(0, |p| p + 1);
            return Some((&rest[..offset], &rest[body_start..]));
        }
    }
    None
}

fn line_start_offset(s: &str, target: usize) -> Option<usize> {
    if target == 0 { return Some(0); }
    let mut cur = 0;
    for (idx, line) in s.lines().enumerate() {
        cur += line.len();
        if s[cur..].starts_with("\r\n") { cur += 2; }
        else if s[cur..].starts_with('\n') { cur += 1; }
        if idx + 1 == target { return Some(cur); }
    }
    None
}

fn parse_yaml(block: &str) -> ObsidianFrontmatter {
    let mut fm = ObsidianFrontmatter::default();

    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((key, value)) = line.split_once(':') else { continue };
        let key = key.trim();
        let value = value.trim();

        match key {
            "title" => fm.title = Some(unquote(value).to_string()),
            "aliases" => fm.aliases = parse_sequence(value),
            "color" => fm.color = Some(unquote(value).to_lowercase()),
            "tags" | "labels" => fm.tags = parse_sequence(value),
            "pinned" | "pin" => fm.pinned = matches!(value, "true" | "yes" | "1"),
            "created" | "date_created" => {
                fm.created = parse_obsidian_date(value);
            }
            "updated" | "modified" | "date_modified" => {
                fm.updated = parse_obsidian_date(value);
            }
            _ => {}
        }
    }

    fm
}

/// Collect inline `#tag` occurrences from the note body (Obsidian-style).
///
/// Only `#word` tokens where the `#` is preceded by whitespace (or is at
/// the start of a line) are treated as tags — `C#` and `#123` are skipped.
pub fn extract_inline_tags(body: &str) -> Vec<String> {
    let mut tags = Vec::new();
    for word in body.split_whitespace() {
        if let Some(tag) = word.strip_prefix('#') {
            if !tag.is_empty() && tag.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) {
                // Strip trailing punctuation
                let tag = tag.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_');
                if !tag.is_empty() {
                    tags.push(tag.to_string());
                }
            }
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

/// Convert Obsidian `[[backlinks]]` to their display text.
///
/// `[[Target]]`      → `Target`
/// `[[Target|Alias]]` → `Alias`
pub fn strip_backlinks(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next(); // consume second '['
            let mut inner = String::new();
            // Collect until ']]'
            loop {
                match chars.next() {
                    None => { out.push_str("[["); out.push_str(&inner); break; }
                    Some(']') if chars.peek() == Some(&']') => {
                        chars.next(); // consume second ']'
                        // If alias present (pipe), use alias; otherwise use target
                        let display = inner
                            .split_once('|')
                            .map(|(_, alias)| alias)
                            .unwrap_or(&inner);
                        out.push_str(display);
                        break;
                    }
                    Some(c) => inner.push(c),
                }
            }
        } else {
            out.push(ch);
        }
    }

    out
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

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

fn parse_sequence(s: &str) -> Vec<String> {
    let s = s.trim();
    // Inline form: [a, b, c] or a, b, c
    let inner = if s.starts_with('[') && s.ends_with(']') { &s[1..s.len() - 1] } else { s };
    inner
        .split(',')
        .map(|item| unquote(item.trim()).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_obsidian_date(s: &str) -> Option<DateTime<Utc>> {
    let s = unquote(s);
    // ISO 8601 / RFC 3339
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.into());
    }
    // Common Obsidian format: "2024-01-15 14:30:00"
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    // Date only: "2024-01-15"
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc());
    }
    None
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_obsidian_fields() {
        let content = "---\ntitle: My Note\naliases: [note, my-note]\ntags: [rust, dev]\ncreated: 2024-01-15\n---\nBody.";
        let (fm, body) = parse(content);
        assert_eq!(fm.title.as_deref(), Some("My Note"));
        assert_eq!(fm.aliases, vec!["note", "my-note"]);
        assert_eq!(fm.tags, vec!["rust", "dev"]);
        assert!(fm.created.is_some());
        assert_eq!(body, "Body.");
    }

    #[test]
    fn strips_backlinks() {
        assert_eq!(strip_backlinks("See [[Other Note]] for details"), "See Other Note for details");
        assert_eq!(strip_backlinks("See [[Target|Alias]] here"), "See Alias here");
    }

    #[test]
    fn extracts_inline_tags() {
        let tags = extract_inline_tags("This is #rust and #programming content #42 C#");
        assert!(tags.contains(&"rust".to_string()));
        assert!(tags.contains(&"programming".to_string()));
        // #42 and C# should be skipped
        assert!(!tags.contains(&"42".to_string()));
    }
}

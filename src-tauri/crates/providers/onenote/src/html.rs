/// Convert OneNote page HTML to plain text for storage in NoteContent::Text.
///
/// OneNote pages are HTML documents. We need to extract readable text for the
/// read-only view. Full WYSIWYG HTML editing is a V4+ feature.
pub fn html_to_text(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    let mut in_style = false;
    let mut in_script = false;
    let mut tag_buf = String::new();
    let mut pending_block = false;

    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_tag {
            if ch == '>' {
                in_tag = false;
                let tag = tag_buf.trim().to_lowercase();
                let tag_name = tag.split_whitespace().next().unwrap_or("");

                // Closing style/script
                if tag_name == "/style" { in_style = false; }
                else if tag_name == "/script" { in_script = false; }
                // Block tags → newline
                else if matches!(
                    tag_name,
                    "p" | "br" | "/p" | "div" | "/div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                    | "/h1" | "/h2" | "/h3" | "/h4" | "/h5" | "/h6"
                    | "li" | "tr"
                ) {
                    pending_block = true;
                }
                // Opening style/script — suppress content
                else if tag_name == "style" { in_style = true; }
                else if tag_name == "script" { in_script = true; }

                tag_buf.clear();
            } else {
                tag_buf.push(ch);
            }
        } else if ch == '<' {
            if pending_block {
                // Trim trailing spaces on the current line before adding newline
                let trimmed = out.trim_end_matches(' ');
                let len = trimmed.len();
                out.truncate(len);
                out.push('\n');
                pending_block = false;
            }
            in_tag = true;
        } else if !in_style && !in_script {
            out.push(ch);
        }
    }

    // Decode HTML entities (named + numeric)
    let decoded = decode_entities(&out);

    // Collapse multiple blank lines to at most two
    let mut result = String::with_capacity(decoded.len());
    let mut blank_count = 0;
    for line in decoded.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            blank_count += 1;
            if blank_count <= 1 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(trimmed);
            result.push('\n');
        }
    }

    result.trim().to_string()
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '&' {
            out.push(ch);
            continue;
        }
        // Collect until ';' or end
        let mut entity = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == ';' { closed = true; break; }
            entity.push(c);
            if entity.len() > 10 { break; }
        }
        if !closed {
            out.push('&');
            out.push_str(&entity);
            continue;
        }
        let decoded = match entity.as_str() {
            "amp"   => "&",
            "lt"    => "<",
            "gt"    => ">",
            "quot"  => "\"",
            "apos"  => "'",
            "nbsp"  => "\u{00a0}",
            "ndash" => "–",
            "mdash" => "—",
            "laquo" => "«",
            "raquo" => "»",
            "aacute" | "Aacute" => if entity.starts_with('A') { "Á" } else { "á" },
            "eacute" | "Eacute" => if entity.starts_with('E') { "É" } else { "é" },
            "iacute" | "Iacute" => if entity.starts_with('I') { "Í" } else { "í" },
            "oacute" | "Oacute" => if entity.starts_with('O') { "Ó" } else { "ó" },
            "uacute" | "Uacute" => if entity.starts_with('U') { "Ú" } else { "ú" },
            "ntilde" | "Ntilde" => if entity.starts_with('N') { "Ñ" } else { "ñ" },
            "uuml"  | "Uuml"   => if entity.starts_with('U') { "Ü" } else { "ü" },
            _ if entity.starts_with('#') => {
                let num_str = &entity[1..];
                let codepoint = if let Some(hex) = num_str.strip_prefix('x').or_else(|| num_str.strip_prefix('X')) {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num_str.parse::<u32>().ok()
                };
                if let Some(cp) = codepoint.and_then(char::from_u32) {
                    out.push(cp);
                } else {
                    out.push('&');
                    out.push_str(&entity);
                    out.push(';');
                }
                continue;
            }
            _ => {
                out.push('&');
                out.push_str(&entity);
                out.push(';');
                continue;
            }
        };
        out.push_str(decoded);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_basic_tags() {
        let html = "<p>Hello <b>world</b></p>";
        let text = html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        assert!(!text.contains('<'));
    }

    #[test]
    fn paragraph_tags_add_newlines() {
        let html = "<p>Line 1</p><p>Line 2</p>";
        let text = html_to_text(html);
        assert!(text.contains("Line 1"));
        assert!(text.contains("Line 2"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(html_to_text(""), "");
    }

    #[test]
    fn decodes_html_entities() {
        let html = "<p>A &amp; B &lt;code&gt;</p>";
        let text = html_to_text(html);
        assert!(text.contains("A & B"));
    }
}

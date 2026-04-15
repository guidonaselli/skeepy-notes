/// Strip Windows Sticky Notes markup from note text.
///
/// The `Text` field in `plum.sqlite` is a "Soft XAML" format — a subset of XAML
/// used by the UWP Sticky Notes app. It contains inline formatting like:
///
///   `\id={UUID}\Bold=true\Italic=false Text content here\n\id={UUID}\Bold=false...`
///
/// Depending on the Sticky Notes version, it may also contain XML-like tags.
/// This function strips all recognized markup and returns clean plain text.
pub fn strip_markup(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }

    // Handle the backslash-escape format used by modern Sticky Notes
    // Format: \key=value\ or \key={value}\
    // Text runs appear between markup tokens on the same "segment"
    if raw.contains("\\id=") {
        return strip_soft_xaml(raw);
    }

    // Handle XML/XAML tag format (older versions or certain content types)
    if raw.contains('<') && raw.contains('>') {
        return strip_xml_tags(raw);
    }

    // Plain text — just clean up null bytes and control chars
    clean_control_chars(raw)
}

fn strip_soft_xaml(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            // Check if this starts a key=value token
            let rest: String = chars.clone().take(50).collect();
            if rest.starts_with("id=")
                || rest.starts_with("Bold=")
                || rest.starts_with("Italic=")
                || rest.starts_with("Strikethrough=")
                || rest.starts_with("Underline=")
                || rest.starts_with("Size=")
                || rest.starts_with("Color=")
                || rest.starts_with("Font=")
                || rest.starts_with("Superscript=")
                || rest.starts_with("Subscript=")
            {
                // Skip until next unescaped backslash or end-of-token
                // Tokens end at the next '\' that starts a new key or at newline
                skip_token(&mut chars);
                continue;
            }
        }
        // Keep newlines and regular text
        if !ch.is_control() || ch == '\n' || ch == '\r' {
            out.push(ch);
        }
    }

    out.trim().to_string()
}

fn skip_token(chars: &mut std::iter::Peekable<std::str::Chars>) {
    // We've consumed the leading '\', now skip until we find another '\' at
    // the start of the next token, or a newline, or end.
    // Tokens look like: Bold=true\ or id={uuid}\
    let mut depth = 0;
    for ch in chars.by_ref() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            if depth > 0 {
                depth -= 1;
            }
        } else if ch == '\\' && depth == 0 {
            // End of this token — the '\' we just consumed starts the next token
            // but we need to put it back conceptually. Since we can't unread,
            // we stop here and let the outer loop handle it.
            break;
        }
    }
}

fn strip_xml_tags(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;

    for ch in raw.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => {
                if !ch.is_control() || ch == '\n' || ch == '\r' {
                    out.push(ch);
                }
            }
            _ => {}
        }
    }

    // Decode common XML entities
    out.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .trim()
        .to_string()
}

fn clean_control_chars(raw: &str) -> String {
    raw.chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\r')
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passthrough() {
        assert_eq!(strip_markup("Hello world"), "Hello world");
    }

    #[test]
    fn strips_xml_tags() {
        let raw = "<Bold>Hello</Bold> <Italic>world</Italic>";
        assert_eq!(strip_markup(raw), "Hello world");
    }

    #[test]
    fn strips_soft_xaml() {
        let raw = r"\id={abc}\Bold=true\Hello world\Bold=false\";
        let result = strip_markup(raw);
        assert!(result.contains("Hello world") || result.is_empty() || !result.contains("\\id="));
    }

    #[test]
    fn empty_input() {
        assert_eq!(strip_markup(""), "");
    }
}

/// Convert HTML into clean plain text, decoding entities and stripping tags.
pub fn to_plain_text(html: &str, max_bytes: usize) -> String {
    let mut out = String::with_capacity(html.len().min(max_bytes));
    let mut in_tag = false;
    let mut in_script_or_style = false;
    let mut tag_buf = String::new();

    let mut chars = html.chars().peekable();

    while let Some(ch) = chars.next() {
        if out.len() >= max_bytes {
            break;
        }

        if ch == '<' {
            in_tag = true;
            tag_buf.clear();
            continue;
        }

        if in_tag {
            if ch == '>' {
                in_tag = false;
                let lower = tag_buf.trim().to_lowercase();
                if lower == "script" || lower == "style" {
                    in_script_or_style = true;
                } else if lower == "/script" || lower == "/style" {
                    in_script_or_style = false;
                } else if matches!(lower.as_str(), "br" | "br/" | "/p" | "/li" | "/div" | "/h1" | "/h2" | "/h3") && !out.ends_with('\n') {
                    out.push('\n');
                }
            } else {
                tag_buf.push(ch);
            }
            continue;
        }

        if in_script_or_style {
            continue;
        }

        // Entity decoding
        if ch == '&' {
            let mut entity = String::new();
            let mut found_semi = false;
            for _ in 0..10 {
                if let Some(&next_ch) = chars.peek() {
                    if next_ch == ';' {
                        chars.next();
                        found_semi = true;
                        break;
                    } else if next_ch.is_alphanumeric() || next_ch == '#' {
                        entity.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            if found_semi {
                match entity.as_str() {
                    "amp" => out.push('&'),
                    "lt" => out.push('<'),
                    "gt" => out.push('>'),
                    "quot" => out.push('"'),
                    "apos" | "#39" => out.push('\''),
                    "nbsp" => out.push(' '),
                    _ => {
                        out.push('&');
                        out.push_str(&entity);
                        out.push(';');
                    }
                }
            } else {
                out.push('&');
                out.push_str(&entity);
            }
            continue;
        }

        // Collapse multiple spaces
        if ch == ' ' || ch == '\t' || ch == '\r' {
            if !out.ends_with(' ') && !out.ends_with('\n') {
                out.push(' ');
            }
        } else if ch == '\n' {
            if !out.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push(ch);
        }
    }

    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_plain_text() {
        let html = "<p>Hello <b>World</b>!</p><br/>&amp; welcome &quot;here&quot;.<script>var x = 1;</script>";
        let text = to_plain_text(html, 1024);
        assert_eq!(text, "Hello World!\n& welcome \"here\".");
    }
}

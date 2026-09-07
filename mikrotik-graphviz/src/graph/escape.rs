/// Append a Graphviz quoted-string-safe value.
pub(super) fn push_dot_escaped(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            _ => output.push(character),
        }
    }
}

/// Append a Graphviz HTML-label-safe value.
pub(super) fn push_html_escaped(output: &mut String, value: &str) {
    for character in value.chars() {
        match character {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            _ => output.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_escaping_handles_quotes_slashes_and_line_endings() {
        let mut output = String::new();
        push_dot_escaped(&mut output, "a\"b\\c\nd\re");
        assert_eq!(output, "a\\\"b\\\\c\\nd\\re");
    }

    #[test]
    fn html_escaping_handles_markup_characters() {
        let mut output = String::new();
        push_html_escaped(&mut output, "<&>\" text");
        assert_eq!(output, "&lt;&amp;&gt;&quot; text");
    }
}

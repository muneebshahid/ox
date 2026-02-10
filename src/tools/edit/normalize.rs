use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LineEnding {
    Lf,
    Crlf,
    Cr,
}

/// Strip UTF-8 BOM if present, returning the BOM and remaining text.
pub fn strip_bom(content: &str) -> (&str, &str) {
    content
        .strip_prefix('\u{FEFF}')
        .map_or(("", content), |rest| ("\u{FEFF}", rest))
}

/// Detect the first line ending style used in the file.
/// Defaults to LF when no line ending is present.
pub fn detect_line_ending(text: &str) -> LineEnding {
    let bytes = text.as_bytes();
    let mut idx = 0;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\r' => {
                if bytes.get(idx + 1) == Some(&b'\n') {
                    return LineEnding::Crlf;
                }
                return LineEnding::Cr;
            }
            b'\n' => return LineEnding::Lf,
            _ => idx += 1,
        }
    }

    LineEnding::Lf
}

/// Normalize all supported line endings to LF.
pub fn normalize_line_endings_to_lf(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Replace unicode special characters with ASCII equivalents and strip
/// trailing whitespace from each line. Preserves final newline if present.
pub fn replace_special_chars(text: &str) -> String {
    text.split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .replace(
            [
                '\u{2018}', // left single curly quote
                '\u{2019}', // right single curly quote
                '\u{201A}', // low single curly quote
                '\u{201B}', // reversed single curly quote
            ],
            "'",
        )
        .replace(
            [
                '\u{201C}', // left double curly quote
                '\u{201D}', // right double curly quote
                '\u{201E}', // low double curly quote
                '\u{201F}', // reversed double curly quote
            ],
            "\"",
        )
        .replace(
            [
                '\u{2010}', // hyphen
                '\u{2011}', // non-breaking hyphen
                '\u{2012}', // figure dash
                '\u{2013}', // en dash
                '\u{2014}', // em dash
                '\u{2015}', // horizontal bar
                '\u{2212}', // minus sign
            ],
            "-",
        )
        .replace(
            [
                '\u{00A0}', // non-breaking space
                '\u{2002}', // en space
                '\u{2003}', // em space
                '\u{2004}', // three-per-em space
                '\u{2005}', // four-per-em space
                '\u{2006}', // six-per-em space
                '\u{2007}', // figure space
                '\u{2008}', // punctuation space
                '\u{2009}', // thin space
                '\u{200A}', // hair space
                '\u{202F}', // narrow non-breaking space
                '\u{205F}', // medium mathematical space
                '\u{3000}', // ideographic (CJK) space
            ],
            " ",
        )
}

/// Restore line endings to the style originally used by the file.
/// Returns a borrowed reference when no conversion is needed.
pub fn restore_line_endings(text: &str, line_ending: LineEnding) -> Cow<'_, str> {
    match line_ending {
        LineEnding::Lf => Cow::Borrowed(text),
        LineEnding::Crlf => Cow::Owned(text.replace('\n', "\r\n")),
        LineEnding::Cr => Cow::Owned(text.replace('\n', "\r")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_line_ending_uses_first_encountered_separator() {
        assert_eq!(detect_line_ending("a\nb\r\nc"), LineEnding::Lf);
        assert_eq!(detect_line_ending("a\r\nb\nc"), LineEnding::Crlf);
        assert_eq!(detect_line_ending("a\rb\nc"), LineEnding::Cr);
    }

    #[test]
    fn detect_line_ending_defaults_to_lf_without_newlines() {
        assert_eq!(detect_line_ending("abc"), LineEnding::Lf);
    }

    #[test]
    fn normalize_line_endings_to_lf_handles_mixed_endings() {
        assert_eq!(normalize_line_endings_to_lf("a\r\nb\rc\n"), "a\nb\nc\n");
    }

    #[test]
    fn restore_line_endings_handles_all_styles() {
        assert_eq!(restore_line_endings("a\nb", LineEnding::Lf), "a\nb");
        assert_eq!(restore_line_endings("a\nb", LineEnding::Crlf), "a\r\nb");
        assert_eq!(restore_line_endings("a\nb", LineEnding::Cr), "a\rb");
    }

    #[test]
    fn replace_special_chars_preserves_final_newline() {
        assert_eq!(replace_special_chars("a  \n"), "a\n");
        assert_eq!(replace_special_chars("a  \n b  \n"), "a\n b\n");
    }
}

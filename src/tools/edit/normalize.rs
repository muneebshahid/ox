use std::borrow::Cow;

/// Strip UTF-8 BOM if present, returning the BOM and remaining text.
pub fn strip_bom(content: &str) -> (&str, &str) {
    content
        .strip_prefix('\u{FEFF}')
        .map_or(("", content), |rest| ("\u{FEFF}", rest))
}

/// Detect whether the file uses CRLF line endings.
pub fn is_crlf(text: &str) -> bool {
    text.contains("\r\n")
}

/// Normalize line endings to LF and replace unicode special characters with
/// ASCII equivalents. Strips trailing whitespace from each line.
pub fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
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

/// Restore line endings to CRLF if the original file used them.
/// Returns a borrowed reference when no conversion is needed.
pub fn restore_line_endings(text: &str, crlf: bool) -> Cow<'_, str> {
    if crlf {
        Cow::Owned(text.replace('\n', "\r\n"))
    } else {
        Cow::Borrowed(text)
    }
}

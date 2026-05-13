use ratatui::style::{Color, Modifier};

/// Each token: (foreground color, bold/italic flags, text chunk).
pub type HighlightToken = (Color, Modifier, Box<str>);

/// One inner Vec per source line, each element is a styled token.
pub type HighlightedLine = Vec<HighlightToken>;

/// Files larger than this are returned as plain text (no highlight).
#[cfg(feature = "syntax-highlight")]
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;

/// Normalize preview text for terminal-safe rendering.
pub(crate) fn normalize_preview_text(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                normalized.push('\n');
            }
            '\n' => normalized.push('\n'),
            '\t' => normalized.push_str("    "),
            ch if ch.is_control() => {}
            ch => normalized.push(ch),
        }
    }

    normalized
}

/// Highlight `text` for the given file `extension` (e.g. `"rs"`, `"py"`).
///
/// When the `syntax-highlight` feature is disabled this always returns `None`
/// so callers fall back to plain `PreviewContent::Text`.
#[cfg(not(feature = "syntax-highlight"))]
pub fn highlight_text(
    _text: &str,
    _extension: Option<&str>,
    _syntect_theme: &str,
) -> Option<Vec<HighlightedLine>> {
    None
}

#[cfg(feature = "syntax-highlight")]
pub fn highlight_text(
    text: &str,
    extension: Option<&str>,
    syntect_theme: &str,
) -> Option<Vec<HighlightedLine>> {
    use std::sync::OnceLock;
    use syntect::easy::HighlightLines;
    use syntect::highlighting::Style as SyntectStyle;
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
    static THEME_SET: OnceLock<two_face::theme::LazyThemeSet> = OnceLock::new();

    fn syntax_set() -> &'static SyntaxSet {
        SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
    }

    fn theme_set() -> &'static two_face::theme::LazyThemeSet {
        THEME_SET.get_or_init(|| two_face::theme::LazyThemeSet::from(two_face::theme::extra()))
    }

    fn to_ratatui_color(c: syntect::highlighting::Color) -> Color {
        Color::Rgb(c.r, c.g, c.b)
    }

    fn to_ratatui_modifier(style: SyntectStyle) -> Modifier {
        use syntect::highlighting::FontStyle;
        let mut m = Modifier::empty();
        if style.font_style.contains(FontStyle::BOLD) {
            m |= Modifier::BOLD;
        }
        if style.font_style.contains(FontStyle::ITALIC) {
            m |= Modifier::ITALIC;
        }
        m
    }

    let text = normalize_preview_text(text);

    if text.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }

    let ss = syntax_set();
    let ts = theme_set();

    let syntax = extension
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = ts
        .get(syntect_theme)
        .or_else(|| ts.get("base16-ocean.dark"))?;

    let mut h = HighlightLines::new(syntax, theme);
    let mut result = Vec::new();

    for line in LinesWithEndings::from(&text) {
        let ranges = h.highlight_line(line, ss).ok()?;
        let tokens: HighlightedLine = ranges
            .into_iter()
            .map(|(style, chunk)| {
                let color = to_ratatui_color(style.foreground);
                let modifier = to_ratatui_modifier(style);
                let text: Box<str> = chunk.trim_end_matches('\n').into();
                (color, modifier, text)
            })
            .filter(|(_, _, t)| !t.is_empty())
            .collect();
        result.push(tokens);
    }

    Some(result)
}

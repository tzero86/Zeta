use std::sync::OnceLock;

use ratatui::style::{Color, Modifier};
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Each token: (foreground color, bold/italic flags, text chunk).
pub type HighlightToken = (Color, Modifier, String);

/// One inner Vec per source line, each element is a styled token.
pub type HighlightedLine = Vec<HighlightToken>;

/// Files larger than this are returned as plain text (no highlight).
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

fn theme_set() -> &'static ThemeSet {
    THEME_SET.get_or_init(ThemeSet::load_defaults)
}

/// Convert a syntect `Color` to a ratatui `Color`.
fn to_ratatui_color(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// Convert a syntect `FontStyle` to a ratatui `Modifier`.
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

/// Highlight `text` for the given file `extension` (e.g. `"rs"`, `"py"`).
///
/// Returns `None` when:
/// - the file exceeds `MAX_HIGHLIGHT_BYTES`, or
/// - the requested syntect theme cannot be found.
///
/// When the extension is unknown the plain-text syntax is used, so the
/// function still returns `Some` (with unstyled tokens) rather than `None`.
/// Callers that receive `None` should fall back to `PreviewContent::Text`.
///
/// `syntect_theme` is a theme name such as `"base16-ocean.dark"`.
pub fn highlight_text(
    text: &str,
    extension: Option<&str>,
    syntect_theme: &str,
) -> Option<Vec<HighlightedLine>> {
    if text.len() > MAX_HIGHLIGHT_BYTES {
        return None;
    }

    let ss = syntax_set();
    let ts = theme_set();

    let syntax = extension
        .and_then(|ext| ss.find_syntax_by_extension(ext))
        .unwrap_or_else(|| ss.find_syntax_plain_text());

    let theme = ts
        .themes
        .get(syntect_theme)
        .or_else(|| ts.themes.get("base16-ocean.dark"))?;

    let mut h = HighlightLines::new(syntax, theme);
    let mut result = Vec::new();

    for line in LinesWithEndings::from(text) {
        let ranges = h.highlight_line(line, ss).ok()?;
        let tokens: HighlightedLine = ranges
            .into_iter()
            .map(|(style, chunk)| {
                let color = to_ratatui_color(style.foreground);
                let modifier = to_ratatui_modifier(style);
                let text = chunk.trim_end_matches('\n').to_string();
                (color, modifier, text)
            })
            .filter(|(_, _, t)| !t.is_empty())
            .collect();
        result.push(tokens);
    }

    Some(result)
}

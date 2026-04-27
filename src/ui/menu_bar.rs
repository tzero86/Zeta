use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::{layout::Rect, Frame};

use crate::action::MenuId;
use crate::config::ThemePalette;
use crate::state::{menu_tabs, AppState, MenuContext};

pub fn render_menu_bar(frame: &mut Frame<'_>, area: Rect, state: &AppState, palette: ThemePalette) {
    let ctx = state.menu_context();
    let bar_is_active = state.active_menu().is_none();
    let top_bar_bg = if bar_is_active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };

    // ── Left side: logo + context badge + menu tabs ────────────────────────
    let mut left_line = Line::default();
    left_line
        .spans
        .extend(top_bar_logo_spans(bar_is_active, palette));
    left_line
        .spans
        .extend(context_badge_spans(ctx, state, palette));
    for tab in menu_tabs(ctx) {
        left_line.spans.extend(menu_spans(
            tab.label,
            Some(tab.mnemonic),
            state.active_menu() == Some(tab.id),
            tab_is_relevant(tab.id, ctx),
            palette,
        ));
    }

    // ── Right side: workspace switcher (right-aligned) ─────────────────────
    let cwd_hint: Option<String> = {
        let cwd = state.active_workspace().panes.active_pane().cwd.clone();
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from);
        let display = if let Some(ref h) = home {
            cwd.strip_prefix(h)
                .map(|r| {
                    let s = r.display().to_string();
                    if s.is_empty() {
                        String::from("~")
                    } else {
                        format!("~/{}", s)
                    }
                })
                .unwrap_or_else(|_| cwd.display().to_string())
        } else {
            cwd.display().to_string()
        };
        let chars: Vec<char> = display.chars().collect();
        if chars.len() > 12 {
            Some(format!(
                "…{}",
                &display[display
                    .char_indices()
                    .nth(chars.len() - 11)
                    .map(|(i, _)| i)
                    .unwrap_or(0)..]
            ))
        } else {
            Some(display)
        }
    };

    let ws_spans = workspace_switcher_spans(
        state.active_workspace_index(),
        state.workspace_count(),
        cwd_hint.as_deref(),
        bar_is_active,
        palette,
    );

    // Measure the workspace switcher's display width to carve out a right slot.
    let ws_width: u16 = ws_spans
        .iter()
        .map(|s| s.content.chars().count() as u16)
        .sum();

    let base_style = Style::default()
        .fg(palette.menu_fg)
        .bg(top_bar_bg)
        .add_modifier(Modifier::BOLD);

    let parts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(ws_width)])
        .split(area);

    frame.render_widget(Paragraph::new(left_line).style(base_style), parts[0]);
    frame.render_widget(
        Paragraph::new(Line::from(ws_spans)).style(base_style),
        parts[1],
    );
}

/// Returns a short badge indicating the current context (editor filename, TERM, etc.).
fn context_badge_spans<'a>(
    ctx: MenuContext,
    state: &AppState,
    palette: ThemePalette,
) -> Vec<Span<'a>> {
    let bg = palette.menu_bg;
    let badge_style = Style::default()
        .fg(palette.logo_accent)
        .bg(bg)
        .add_modifier(Modifier::BOLD);

    match ctx {
        MenuContext::Pane => vec![],
        MenuContext::Terminal => vec![
            Span::styled(" ● ", badge_style),
            Span::styled("TERM ", Style::default().fg(palette.menu_fg).bg(bg)),
        ],
        MenuContext::TerminalFullscreen => vec![
            Span::styled(" ◈ ", badge_style),
            Span::styled("TERMINAL ", Style::default().fg(palette.menu_fg).bg(bg)),
        ],
        MenuContext::Editor | MenuContext::EditorFullscreen => {
            let prefix = if ctx == MenuContext::EditorFullscreen {
                " ◈ EDITOR "
            } else {
                " ◈ "
            };
            let filename: String = state
                .editor()
                .and_then(|e| e.path.as_ref())
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| String::from("[untitled]"));
            vec![
                Span::styled(prefix, badge_style),
                Span::styled(
                    format!("{filename} "),
                    Style::default().fg(palette.menu_fg).bg(bg),
                ),
            ]
        }
    }
}

pub fn top_bar_logo_spans(active: bool, palette: ThemePalette) -> Vec<Span<'static>> {
    let bg = if active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let bracket_style = Style::default()
        .fg(palette.logo_accent)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let letter_style = Style::default()
        .fg(palette.logo_accent)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let name_style = Style::default()
        .fg(palette.menu_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);

    vec![
        Span::styled(" ", Style::default().bg(bg)),
        Span::styled("[", bracket_style),
        Span::styled("Z", letter_style),
        Span::styled("]", bracket_style),
        Span::styled("eta ", name_style),
    ]
}

pub fn workspace_switcher_spans(
    active_workspace: usize,
    workspace_count: usize,
    cwd_hint: Option<&str>,
    bar_active: bool,
    palette: ThemePalette,
) -> Vec<Span<'static>> {
    let bg = if bar_active {
        palette.menu_active_bg
    } else {
        palette.menu_bg
    };
    let inactive_style = Style::default()
        .fg(palette.menu_fg)
        .bg(bg)
        .add_modifier(Modifier::BOLD);
    let active_style = Style::default()
        .fg(palette.selection_fg)
        .bg(palette.selection_bg)
        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED | Modifier::REVERSED);
    let mut spans = Vec::with_capacity(workspace_count.saturating_mul(2) + 1);
    spans.push(Span::styled(" ", Style::default().bg(bg)));
    for idx in 0..workspace_count {
        let style = if idx == active_workspace {
            active_style
        } else {
            inactive_style
        };
        let label = if idx == active_workspace {
            if let Some(hint) = cwd_hint {
                format!(" {}:{} ", idx + 1, hint)
            } else {
                format!(" {} ", idx + 1)
            }
        } else {
            format!(" {} ", idx + 1)
        };
        spans.push(Span::styled(label, style));
        spans.push(Span::styled(" ", Style::default().bg(bg)));
    }
    spans
}

fn menu_spans(
    label: &'static str,
    mnemonic: Option<char>,
    active: bool,
    is_relevant: bool,
    palette: ThemePalette,
) -> Vec<Span<'static>> {
    let fg = if !is_relevant {
        palette.text_muted
    } else {
        palette.menu_fg
    };
    let style = if active {
        Style::default()
            .fg(fg)
            .bg(palette.menu_active_bg)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().fg(fg).bg(palette.menu_bg)
    };

    let highlighted = mnemonic.map(|value| value.to_ascii_uppercase());
    let mut spans = Vec::with_capacity(label.len());
    let mut used_highlight = false;

    for ch in label.chars() {
        let mut char_style = style;
        if is_relevant && !used_highlight && Some(ch.to_ascii_uppercase()) == highlighted {
            char_style = char_style.fg(palette.menu_mnemonic_fg);
            used_highlight = true;
        }
        spans.push(Span::styled(ch.to_string(), char_style));
    }

    spans
}

fn tab_is_relevant(tab_id: MenuId, ctx: MenuContext) -> bool {
    match ctx {
        MenuContext::Pane => matches!(
            tab_id,
            MenuId::File | MenuId::Navigate | MenuId::View | MenuId::Help
        ),
        MenuContext::Editor | MenuContext::EditorFullscreen => true,
        MenuContext::Terminal | MenuContext::TerminalFullscreen => {
            matches!(tab_id, MenuId::Navigate | MenuId::View | MenuId::Help)
        }
    }
}

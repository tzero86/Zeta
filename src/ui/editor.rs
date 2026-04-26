use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::editor::{EditorBuffer, EditorRenderState};
use crate::ui::code_view::{
    render_code_view, CodeViewRenderArgs, SearchHighlight, SelectionHighlight,
};
use crate::ui::styles::{
    dirty_indicator_style, panel_title_focused_style, panel_title_unfocused_style,
};

pub struct RenderEditorArgs<'a> {
    pub editor: &'a mut EditorBuffer,
    pub render_state: &'a EditorRenderState,
    pub is_focused: bool,
    pub palette: ThemePalette,
    pub syntect_theme: &'a str,
    pub replace_active: bool,
    pub replace_query: &'a str,
    pub loading: bool,
    pub cheap_mode: bool,
    /// Tab width used in cheap_mode and highlighted rendering.
    pub cheap_tab_width: u8,
}

pub fn editor_render_state(
    editor: &mut EditorBuffer,
    area: Rect,
    is_active: bool,
    tab_width: u8,
    word_wrap: bool,
) -> EditorRenderState {
    let viewport_cols = area.width.saturating_sub(6) as usize;
    let viewport_rows = area.height.saturating_sub(2) as usize;
    editor.render_state(
        viewport_rows,
        viewport_cols,
        is_active,
        tab_width,
        word_wrap,
    )
}

pub fn editor_highlighted_render_state(
    editor: &mut EditorBuffer,
    area: Rect,
    syntect_theme: &str,
    palette: ThemePalette,
    tab_width: u8,
    word_wrap: bool,
) -> (usize, Vec<crate::highlight::HighlightedLine>) {
    let height = area.height.saturating_sub(2) as usize;
    // Subtract gutter_width (6) so wrapping aligns with what code_view renders.
    let viewport_cols = area.width.saturating_sub(6) as usize;
    let lines = editor.visible_highlighted_window(
        height,
        syntect_theme,
        palette.text_primary,
        tab_width,
        word_wrap,
        viewport_cols,
    );
    // visible_start is the first logical line shown; use it as the line-number base.
    let visible_start = editor
        .render_state(height, viewport_cols, false, tab_width, word_wrap)
        .visible_start;
    (visible_start, lines)
}

pub fn render_editor(frame: &mut Frame<'_>, area: Rect, args: RenderEditorArgs<'_>) {
    let RenderEditorArgs {
        editor,
        render_state,
        is_focused,
        palette,
        syntect_theme,
        replace_active,
        replace_query,
        loading,
        cheap_mode,
        cheap_tab_width,
    } = args;
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_editor_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };

    let filename = editor
        .path
        .as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| String::from("[untitled]"));
    let parent = editor
        .path
        .as_ref()
        .and_then(|p| p.parent())
        .and_then(|d| d.file_name())
        .map(|n| format!("{}/", n.to_string_lossy()))
        .unwrap_or_default();
    let dirty_part = if editor.is_dirty { " ● " } else { "   " };
    let (cursor_line, cursor_col) = editor.cursor_line_col();
    let ln_col = format!(" Ln {} · Col {} ", cursor_line + 1, cursor_col + 1);
    let accent = palette.border_editor_focus;
    let title_style = if is_focused {
        panel_title_focused_style(accent)
    } else {
        panel_title_unfocused_style(palette)
    };
    let dirty_style = if editor.is_dirty {
        dirty_indicator_style(palette)
    } else {
        title_style
    };
    let badge_style = Style::default()
        .fg(palette.surface_bg)
        .bg(accent)
        .add_modifier(Modifier::BOLD);
    let title_line = Line::from(vec![
        Span::styled(format!(" \u{f0187} {} ", filename), title_style),
        Span::styled(dirty_part, dirty_style),
        Span::styled(
            format!(" {} ", parent),
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.surface_bg),
        ),
        Span::styled(ln_col, Style::default().fg(palette.text_subtext)),
        Span::styled(" Editor ", badge_style),
    ]);
    let block = Block::default()
        .title(title_line)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (content_area, search_bar_area, replace_bar_area) =
        if editor.search_active && replace_active {
            let splits = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(inner);
            (splits[0], Some(splits[1]), Some(splits[2]))
        } else if editor.search_active {
            let splits = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(1)])
                .split(inner);
            (splits[0], Some(splits[1]), None)
        } else {
            (inner, None, None)
        };

    let gutter_width = 6u16;
    let sel_row_ranges = editor.visible_selection_display_ranges(
        render_state.visible_start,
        &render_state.visible_lines,
        cheap_tab_width,
        render_state.word_wrap,
    );
    let has_selection = editor.selection_range().is_some();
    if loading {
        let loading_text = editor
            .path
            .as_ref()
            .map(|p| format!("Loading {}...", p.display()))
            .unwrap_or_else(|| String::from("Loading..."));
        let loading = Paragraph::new(loading_text).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.surface_bg),
        );
        frame.render_widget(loading, content_area);
    } else if cheap_mode {
        let plain_lines: Vec<crate::highlight::HighlightedLine> = render_state
            .visible_lines
            .iter()
            .cloned()
            .map(|line| vec![(palette.text_primary, Modifier::empty(), line.into())])
            .collect();
        render_code_view(
            frame,
            content_area,
            CodeViewRenderArgs {
                lines: &plain_lines,
                first_line_number: render_state.visible_start + 1,
                gutter_width,
                scroll_col: render_state.scroll_col,
                cursor_row: None,
                palette,
                search: None,
                selection: if has_selection {
                    Some(SelectionHighlight {
                        row_ranges: &sel_row_ranges,
                        bg: palette.text_sel_bg,
                    })
                } else {
                    None
                },
            },
        );
    } else {
        // Compute per-row search match ranges for highlight overlay.
        let (search_row_matches, active_row_match) = editor
            .visible_search_matches(&render_state.visible_lines, render_state.cursor_visible_row);
        let search_highlight = if editor.search_active && !editor.search_query.is_empty() {
            Some(SearchHighlight {
                row_matches: &search_row_matches,
                active_row_match,
            })
        } else {
            None
        };
        let (first_line_num, highlighted) = editor_highlighted_render_state(
            editor,
            content_area,
            syntect_theme,
            palette,
            cheap_tab_width,
            render_state.word_wrap,
        );
        render_code_view(
            frame,
            content_area,
            CodeViewRenderArgs {
                lines: &highlighted,
                first_line_number: first_line_num + 1,
                gutter_width,
                scroll_col: render_state.scroll_col,
                cursor_row: render_state.cursor_visible_row,
                palette,
                search: search_highlight,
                selection: if has_selection {
                    Some(SelectionHighlight {
                        row_ranges: &sel_row_ranges,
                        bg: palette.text_sel_bg,
                    })
                } else {
                    None
                },
            },
        );
    }

    if let Some(bar_area) = search_bar_area {
        let matches = editor.find_matches(&editor.search_query.clone());
        let count_str = if editor.search_query.is_empty() {
            String::new()
        } else if matches.is_empty() {
            String::from("  0/0")
        } else {
            let current = editor.search_match_idx.min(matches.len() - 1) + 1;
            format!("  {current}/{count}", count = matches.len())
        };
        let bar_text = format!(
            " Find: {}{}  [Enter/F3 next  Shift+F3 prev  Esc close]",
            editor.search_query, count_str
        );
        let bar = Paragraph::new(bar_text).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.selection_bg),
        );
        frame.render_widget(bar, bar_area);
    }

    if let Some(bar_area) = replace_bar_area {
        let bar_text = format!(
            " Replace: {}  [Ctrl+H replace  Ctrl+Shift+H all]",
            replace_query
        );
        let bar = Paragraph::new(bar_text).style(
            Style::default()
                .fg(palette.text_primary)
                .bg(palette.tools_bg),
        );
        frame.render_widget(bar, bar_area);
    }

    if !loading && !cheap_mode {
        if let Some(cursor_visual_row) = render_state.cursor_visible_row {
            let content_x = content_area.x + gutter_width;
            let cursor_y = content_area.y
                + (cursor_visual_row as u16).min(content_area.height.saturating_sub(1));
            let viewport_cols = content_area.width.saturating_sub(gutter_width) as usize;
            let visual_col = render_state.cursor_visual_col.unwrap_or_else(|| {
                // Fallback: raw char col (no tab expansion).
                editor.cursor_line_col().1
            });
            let visible_col = if render_state.word_wrap && viewport_cols > 0 {
                visual_col % viewport_cols
            } else {
                visual_col.saturating_sub(render_state.scroll_col)
            };
            let cursor_x = content_x
                + (visible_col as u16).min(content_area.width.saturating_sub(gutter_width + 1));
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

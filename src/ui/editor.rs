use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::config::ThemePalette;
use crate::editor::{EditorBuffer, EditorRenderState};
use crate::ui::code_view::{render_code_view, CodeViewRenderArgs};

pub fn editor_render_state(
    editor: &mut EditorBuffer,
    area: Rect,
    is_active: bool,
) -> EditorRenderState {
    let viewport_cols = area.width.saturating_sub(6) as usize;
    let viewport_rows = area.height.saturating_sub(2) as usize;
    editor.render_state(viewport_rows, viewport_cols, is_active)
}

pub fn editor_highlighted_render_state(
    editor: &EditorBuffer,
    area: Rect,
    syntect_theme: &str,
    palette: ThemePalette,
) -> (usize, Vec<crate::highlight::HighlightedLine>) {
    let height = area.height.saturating_sub(2) as usize;
    editor.visible_highlighted_window(height, syntect_theme, palette.text_primary)
}

pub fn render_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    editor: &mut EditorBuffer,
    render_state: &EditorRenderState,
    is_focused: bool,
    palette: ThemePalette,
    syntect_theme: &str,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(palette.border_editor_focus)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette.text_muted)
    };

    let path = editor
        .path
        .as_ref()
        .map(|value| value.display().to_string())
        .unwrap_or_else(|| String::from("<unnamed>"));
    let dirty_marker = if editor.is_dirty { "*" } else { "" };
    let title = format!("Editor{}  {}", dirty_marker, path);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let (content_area, search_bar_area) = if editor.search_active {
        let splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (splits[0], Some(splits[1]))
    } else {
        (inner, None)
    };

    let gutter_width = 6u16;
    let (first_line_num, highlighted) =
        editor_highlighted_render_state(editor, content_area, syntect_theme, palette);

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
        },
    );

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

    if render_state.cursor_visible_row.is_some() {
        let (line, column) = editor.cursor_line_col();
        let visible_line = line.saturating_sub(render_state.visible_start);
        let content_x = content_area.x + gutter_width;
        let cursor_y =
            content_area.y + (visible_line as u16).min(content_area.height.saturating_sub(1));
        let visible_col = column.saturating_sub(render_state.scroll_col);
        let cursor_x = content_x
            + (visible_col as u16).min(content_area.width.saturating_sub(gutter_width + 1));
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

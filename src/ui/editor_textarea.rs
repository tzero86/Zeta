use crate::editor_textarea::TextAreaAdapter;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::Paragraph;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    Frame,
};

pub struct RenderTextareaEditorArgs<'a> {
    pub editor: &'a mut TextAreaAdapter,
    pub area: Rect,
    pub is_focused: bool,
    pub show_search_bar: bool,
    pub line_number_color: ratatui::style::Color,
}

pub fn render_textarea_editor(frame: &mut Frame, args: RenderTextareaEditorArgs) {
    let area = args.area;

    // If search bar is active, split area: top = editor, bottom = search bar (1 line)
    let editor_area = if args.show_search_bar {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        // Render search bar in bottom chunk
        render_search_bar(frame, args.editor, chunks[1]);
        chunks[0]
    } else {
        area
    };

    args.editor
        .render(frame, editor_area, args.is_focused, args.line_number_color);
}

fn render_search_bar(frame: &mut Frame, editor: &TextAreaAdapter, area: Rect) {
    let query = &editor.search_query;
    let match_count = editor.match_count();
    let current = if match_count > 0 {
        editor.current_match_idx() + 1
    } else {
        0
    };
    let text = format!("Search: {} ({}/{})", query, current, match_count);

    frame.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::Yellow)),
        area,
    );
}

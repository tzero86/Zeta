use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::fs::EntryInfo;
use crate::pane::{PaneId, PaneState};
use crate::state::AppState;

pub fn render(frame: &mut Frame<'_>, state: &AppState) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(areas[0]);

    render_pane(
        frame,
        panes[0],
        state.left_pane(),
        state.focus() == PaneId::Left,
    );
    render_pane(
        frame,
        panes[1],
        state.right_pane(),
        state.focus() == PaneId::Right,
    );

    let status = Paragraph::new(Line::raw(state.status_line())).style(
        Style::default()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(status, areas[1]);
}

fn render_pane(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    pane: &PaneState,
    is_focused: bool,
) {
    let border_style = if is_focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let title = format!("{}  {}", pane.title, pane.cwd.display());
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem<'_>> = if pane.entries.is_empty() {
        vec![ListItem::new("(empty)")]
    } else {
        pane.entries.iter().map(render_item).collect()
    };

    let list = List::new(items)
        .highlight_style(Style::default().bg(Color::DarkGray).fg(Color::White))
        .highlight_symbol("> ");

    let mut list_state = ListState::default();
    if !pane.entries.is_empty() {
        list_state.select(Some(pane.selection));
    }

    frame.render_stateful_widget(list, inner, &mut list_state);
}

fn render_item(entry: &EntryInfo) -> ListItem<'static> {
    let line = format!("{} {}", entry.kind.symbol(), entry.name);
    ListItem::new(line)
}

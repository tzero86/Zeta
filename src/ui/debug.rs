//! Floating debug panel — toggled with F12.
//!
//! Shows live state: focus layer, last key event, last action dispatched,
//! pane split ratio, active directory, and a rolling action log.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::state::AppState;

const PANEL_WIDTH: u16 = 46;
const PANEL_HEIGHT: u16 = 17;

/// Render the debug overlay in the top-right corner of `area`.
/// Call this last so it always appears above other widgets.
pub fn render_debug_panel(frame: &mut Frame, area: Rect, state: &AppState) {
    if !state.debug_visible {
        return;
    }

    let x = area.right().saturating_sub(PANEL_WIDTH);
    let y = area.y + 1; // one row below the menu bar
    let width = PANEL_WIDTH.min(area.width);
    let height = PANEL_HEIGHT.min(area.height.saturating_sub(2));
    let panel = Rect::new(x, y, width, height);

    let border_style = Style::default().fg(Color::Yellow);
    let label_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let value_style = Style::default().fg(Color::White);
    let dim_style = Style::default().fg(Color::DarkGray);
    let log_style = Style::default().fg(Color::Cyan);

    // Erase whatever was behind the panel.
    frame.render_widget(Clear, panel);

    let block = Block::default()
        .title(Span::styled(" ⚙ Debug  F12 ", label_style))
        .borders(Borders::ALL)
        .border_style(border_style);

    let inner = block.inner(panel);
    frame.render_widget(block, panel);

    // ── Build rows ──────────────────────────────────────────────────────────
    let focus_str = format!("{:?}", state.focus_layer());
    let ratio = state.pane_split_ratio();
    let cwd = state
        .panes
        .active_pane()
        .cwd
        .to_string_lossy()
        .to_string();
    let cwd_short = if cwd.len() > (inner.width as usize).saturating_sub(10) {
        format!("…{}", &cwd[cwd.len().saturating_sub((inner.width as usize).saturating_sub(11))..])
    } else {
        cwd
    };

    let selected = state
        .panes
        .active_pane()
        .selected_path()
        .map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
        .unwrap_or_else(|| "—".into());

    let ws_idx = state.active_workspace_index() + 1;
    let redraws = state.redraw_count();
    let ratio_str = format!("{ratio}%");
    let ws_str = format!("{ws_idx}/4");
    let redraws_str = redraws.to_string();

    let mut rows: Vec<Line> = vec![
        row(label_style, value_style, "Focus", &focus_str),
        row(label_style, value_style, "Last key", &state.debug.last_key),
        row(label_style, value_style, "Last action", &state.debug.last_action),
        row(label_style, value_style, "Split ratio", &ratio_str),
        row(label_style, value_style, "Workspace", &ws_str),
        row(label_style, value_style, "CWD", &cwd_short),
        row(label_style, value_style, "Selection", &selected),
        row(label_style, value_style, "Redraws", &redraws_str),
        Line::from(Span::styled(
            "─── Action log ──────────────────────",
            dim_style,
        )),
    ];

    if state.debug.action_log.is_empty() {
        rows.push(Line::from(Span::styled("  (no actions yet)", dim_style)));
    } else {
        for entry in &state.debug.action_log {
            rows.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(entry.as_str(), log_style),
            ]));
        }
    }

    let para = Paragraph::new(rows);
    frame.render_widget(para, inner);

    // Key hint at the very bottom of the panel.
    if inner.height > 1 {
        let hint = Line::from(Span::styled(
            " F12 close ",
            Style::default().fg(Color::DarkGray),
        ));
        let hint_widget = Paragraph::new(hint);
        let hint_rect = Rect::new(
            inner.x,
            inner.y + inner.height.saturating_sub(1),
            inner.width,
            1,
        );
        frame.render_widget(hint_widget, hint_rect);
    }
}

fn row<'a>(
    label_style: Style,
    value_style: Style,
    label: &'a str,
    value: &'a str,
) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<12} "), label_style),
        Span::styled(value.to_string(), value_style),
    ])
}

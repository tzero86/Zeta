mod bookmarks;
mod code_view;
mod debug;
mod editor;
mod finder;
pub(crate) mod highlight;
pub mod markdown;
mod menu_bar;
mod overlay;
mod palette;
mod pane;
pub mod preview;
mod settings;
pub mod ssh;
mod styles;
pub mod terminal;

pub mod layout_cache;
pub use layout_cache::LayoutCache;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::pane::PaneId;
use crate::state::{AppState, PaneLayout};
use crate::ui::bookmarks::render_bookmarks_modal;
use crate::ui::editor::{editor_render_state, render_editor, RenderEditorArgs};
use crate::ui::finder::render_file_finder;
use crate::ui::markdown::{parse_markdown_lines_with_palette, render_md_with_lines};
use crate::ui::menu_bar::render_menu_bar;
use crate::ui::overlay::{
    menu_popup_width, render_collision_dialog, render_destructive_confirm, render_dialog,
    render_menu_popup, render_open_with_popup, render_prompt,
};
use crate::ui::palette::render_command_palette;
use crate::ui::pane::{render_pane, RenderPaneArgs};
use crate::ui::preview::{render_preview_panel, RenderPreviewArgs};
use crate::ui::settings::render_settings_panel;
use crate::ui::ssh::{render_ssh_connect_dialog, render_ssh_trust_prompt};

use ratatui::widgets::Borders;

/// Render the full TUI. Returns a `LayoutCache` recording each panel's `Rect`
/// so the event loop can use it for mouse hit-testing without re-running layout.
pub fn render(frame: &mut Frame<'_>, state: &mut AppState) -> LayoutCache {
    let palette = state.theme().palette;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_menu_bar(frame, areas[0], state, palette);

    let pane_direction = match state.pane_layout() {
        PaneLayout::SideBySide => Direction::Horizontal,
        PaneLayout::Stacked => Direction::Vertical,
    };

    let is_preview_open = state.is_preview_panel_open();
    let has_editor = state.editor().is_some();
    let editor_fullscreen = has_editor && state.is_editor_fullscreen();
    let show_md_preview = has_editor && state.is_markdown_preview_visible();
    let pane_navigation_mode = matches!(
        state.focus_layer(),
        crate::state::FocusLayer::Pane | crate::state::FocusLayer::PaneFilter
    );
    let cheap_tools_mode = !editor_fullscreen && pane_navigation_mode;
    let show_tools = has_editor || is_preview_open;

    let tools_pct = if has_editor { 50u16 } else { 40u16 };
    let panes_pct = 100 - tools_pct;

    let (main_content_area, terminal_area) = if state.terminal.is_open() {
        if state.is_terminal_fullscreen() {
            // Full content area goes to the terminal; panes/editor are hidden.
            (Rect::default(), Some(areas[1]))
        } else {
            let splits = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Percentage(30)])
                .split(areas[1]);
            (splits[0], Some(splits[1]))
        }
    } else {
        (areas[1], None)
    };

    if let Some(t_area) = terminal_area {
        let focused = state.focus_layer() == crate::state::FocusLayer::Terminal;
        crate::ui::terminal::render_terminal(frame, t_area, &state.terminal, palette, focused);
    }

    let (pane_area, tools_area_opt) = if editor_fullscreen {
        (Rect::default(), Some(main_content_area))
    } else if show_tools {
        let vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(panes_pct),
                Constraint::Percentage(tools_pct),
            ])
            .split(main_content_area);
        (vertical[0], Some(vertical[1]))
    } else {
        (main_content_area, None)
    };

    let mut left_pane_rect = Rect::default();
    let mut right_pane_rect = Rect::default();
    let mut editor_panel_rect = None;
    let mut file_preview_panel_rect = None;
    let mut markdown_preview_panel_rect = None;

    if !editor_fullscreen {
        let ratio = state.pane_split_ratio() as u16;
        let panes = Layout::default()
            .direction(pane_direction)
            .constraints([
                Constraint::Percentage(ratio),
                Constraint::Percentage(100 - ratio),
            ])
            .split(pane_area);

        left_pane_rect = panes[0];
        right_pane_rect = panes[1];

        let left_focused = state.focus() == PaneId::Left;
        let right_focused = state.focus() == PaneId::Right;

        let is_stacked = state.pane_layout() == PaneLayout::Stacked;
        let (first_label, second_label) = if is_stacked {
            ("Top", "Bottom")
        } else {
            ("Left", "Right")
        };

        render_pane(
            frame,
            panes[0],
            RenderPaneArgs {
                pane: state.left_pane(),
                label: first_label,
                is_focused: left_focused,
                is_left: true,
                borders: Borders::TOP | Borders::LEFT | Borders::BOTTOM,
                state,
                git: state.git_status(PaneId::Left),
            },
        );

        render_pane(
            frame,
            panes[1],
            RenderPaneArgs {
                pane: state.right_pane(),
                label: second_label,
                is_focused: right_focused,
                is_left: false,
                borders: Borders::ALL,
                state,
                git: state.git_status(PaneId::Right),
            },
        );
    }

    if let Some(tools_area) = tools_area_opt {
        if has_editor {
            let editor_focused = state.is_editor_focused();
            let editor_loading = state.is_editor_loading();
            let md_focused = state.is_markdown_preview_focused();
            let md_scroll = state.markdown_preview_scroll();
            let replace_active = state.editor.replace_active;
            let replace_query = state.editor.replace_query.clone();
            let syntect_theme = state.theme().palette.syntect_theme;
            let (editor_area, md_area_opt) = if show_md_preview {
                let halves = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(tools_area);
                (halves[0], Some(halves[1]))
            } else {
                (tools_area, None)
            };
            editor_panel_rect = Some(editor_area);
            markdown_preview_panel_rect = md_area_opt;
            let ed_cfg = state.config().editor.clone();

            if let Some(editor) = state.editor_mut() {
                let editor_view = editor_render_state(
                    editor,
                    editor_area,
                    editor_focused,
                    ed_cfg.tab_width,
                    ed_cfg.word_wrap,
                );
                render_editor(
                    frame,
                    editor_area,
                    RenderEditorArgs {
                        editor,
                        render_state: &editor_view,
                        is_focused: editor_focused,
                        palette,
                        syntect_theme,
                        replace_active,
                        replace_query: &replace_query,
                        loading: editor_loading,
                        cheap_mode: cheap_tools_mode && !editor_focused,
                        cheap_tab_width: ed_cfg.tab_width,
                    },
                );

                if let Some(md_area) = md_area_opt {
                    let source = editor.contents();
                    if cheap_tools_mode && !md_focused {
                        let text = source
                            .lines()
                            .take(md_area.height.saturating_sub(2) as usize)
                            .collect::<Vec<_>>()
                            .join("\n");
                        let block = ratatui::widgets::Block::default()
                            .title(" Markdown Preview ")
                            .borders(ratatui::widgets::Borders::ALL)
                            .border_style(ratatui::style::Style::default().fg(palette.text_muted))
                            .style(ratatui::style::Style::default().bg(palette.tools_bg));
                        let inner = block.inner(md_area);
                        frame.render_widget(block, md_area);
                        frame.render_widget(
                            ratatui::widgets::Paragraph::new(text)
                                .style(ratatui::style::Style::default().bg(palette.tools_bg)),
                            inner,
                        );
                    } else {
                        let inner_width = md_area.width.saturating_sub(2);
                        if editor
                            .md_preview_cached(inner_width, syntect_theme)
                            .is_none()
                        {
                            let parsed =
                                parse_markdown_lines_with_palette(&source, palette, inner_width);
                            editor.set_md_preview_cache(inner_width, syntect_theme, parsed);
                        }
                        render_md_with_lines(
                            frame,
                            md_area,
                            editor
                                .md_preview_cached(inner_width, syntect_theme)
                                .unwrap(),
                            palette,
                            md_scroll,
                            md_focused,
                        );
                    }
                }
            }
        } else if is_preview_open {
            let preview_view = state.preview_view().map(|(_, view)| view);
            let filename = state.active_pane_title().to_string();
            file_preview_panel_rect = Some(tools_area);
            render_preview_panel(
                frame,
                tools_area,
                RenderPreviewArgs {
                    view: preview_view,
                    filename: &filename,
                    is_focused: state.is_preview_focused(),
                    palette,
                    // cheap_mode is intentionally disabled for the preview panel.
                    // The highlighting work is done in the background worker; rendering
                    // already-highlighted spans as colored ratatui Spans costs less than
                    // the string-building that cheap_mode performs per frame.
                    cheap_mode: false,
                },
            );
        }
    }

    let mut menu_popup_rect: Option<Rect> = None;
    if let Some(menu) = state.active_menu() {
        let item_count = state.menu_items().len();
        let menu_ctx = state.menu_context();
        let mut popup_x = areas[0].x + 1;
        let mut cursor = areas[0].x + 8;
        for tab in crate::state::menu_tabs(menu_ctx) {
            if tab.id == menu {
                popup_x = cursor;
                break;
            }
            cursor += tab.label.len() as u16;
        }
        let rect = Rect {
            x: popup_x,
            y: areas[1].y,
            width: menu_popup_width(&state.menu_items()),
            height: item_count as u16 + 2,
        };
        menu_popup_rect = Some(rect);
        render_menu_popup(
            frame,
            areas[1],
            menu,
            &state.menu_items(),
            state.menu_selection(),
            palette,
            menu_ctx,
        );
    }

    if let Some(prompt) = state.prompt() {
        render_prompt(frame, areas[1], prompt, palette);
    }

    if let Some(dialog) = state.dialog() {
        render_dialog(frame, areas[1], dialog, palette);
    }

    if let Some(collision) = state.collision() {
        render_collision_dialog(frame, areas[1], collision, palette);
    }

    if let Some(destructive_state) = state.destructive_confirm() {
        render_destructive_confirm(frame, areas[1], destructive_state, palette);
    }

    if let Some(palette_state) = state.palette() {
        render_command_palette(frame, areas[1], palette_state, palette);
    }

    if let Some(settings_state) = state.settings() {
        render_settings_panel(frame, areas[1], settings_state, state, palette);
    }

    if let Some(bookmarks_state) = state.bookmarks() {
        render_bookmarks_modal(
            frame,
            areas[1],
            bookmarks_state,
            &state.config().bookmarks,
            palette,
        );
    }

    if let Some(finder_state) = state.file_finder() {
        render_file_finder(frame, areas[1], finder_state, palette);
    }

    if let Some(ssh_state) = state.ssh_connect() {
        render_ssh_connect_dialog(frame, areas[1], ssh_state, &palette);
    }

    if let Some(crate::state::overlay::ModalState::SshTrustPrompt {
        host,
        port,
        fingerprints,
        ..
    }) = &state.overlay.modal
    {
        render_ssh_trust_prompt(frame, areas[1], host, *port, fingerprints, &palette);
    }

    if let Some((items, selection, _target)) = state.overlay.open_with() {
        render_open_with_popup(frame, areas[1], items, selection, palette);
    }

    render_status_bar(frame, areas[2], state, palette);
    render_key_hints(frame, areas[3], state, palette);

    // Debug panel renders last so it always floats above everything else.
    debug::render_debug_panel(frame, areas[1], state);

    LayoutCache {
        menu_bar: areas[0],
        left_pane: left_pane_rect,
        right_pane: right_pane_rect,
        tools_panel: tools_area_opt,
        editor_panel: editor_panel_rect,
        file_preview_panel: file_preview_panel_rect,
        markdown_preview_panel: markdown_preview_panel_rect,
        status_bar: areas[2],
        hint_bar: areas[3],
        menu_popup: menu_popup_rect,
        terminal_panel: terminal_area,
    }
}

fn render_status_bar(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    palette: crate::config::ThemePalette,
) {
    let zones = state.status_zones();
    let mut spans = Vec::new();

    if let Some(ref progress) = zones.progress {
        let op_text = format!(
            " {} {}/{} — {} ",
            progress.operation, progress.current, progress.total, progress.current_name,
        );
        let bar_width = (area.width as usize).saturating_sub(op_text.len() + 2);
        let filled = if progress.total > 0 {
            (progress.current * bar_width as u64 / progress.total) as usize
        } else {
            0
        };
        let empty = bar_width.saturating_sub(filled);
        spans.push(Span::styled(
            format!("{}{}{} ", op_text, "─".repeat(filled), "░".repeat(empty)),
            Style::default().fg(palette.status_fg).bg(palette.status_bg),
        ));
    } else {
        if let Some(ref git) = zones.git_branch {
            spans.push(Span::styled(
                git.clone(),
                Style::default()
                    .fg(palette.border_focus)
                    .bg(palette.status_git_bg),
            ));
            spans.push(Span::styled(
                "│",
                Style::default()
                    .fg(palette.text_muted)
                    .bg(palette.status_bg),
            ));
        }
        if let Some(ref entry) = zones.entry_detail {
            spans.push(Span::styled(
                entry.clone(),
                Style::default()
                    .fg(palette.text_primary)
                    .bg(palette.status_entry_bg),
            ));
            spans.push(Span::styled(
                "│",
                Style::default()
                    .fg(palette.text_muted)
                    .bg(palette.status_bg),
            ));
        }
        spans.push(Span::styled(
            zones.message.clone(),
            Style::default()
                .fg(palette.text_subtext)
                .bg(palette.status_bg),
        ));
        if let Some(ref marks) = zones.marks {
            let size_str = if marks.total_bytes > 0 {
                format!(" ✦ {} · {} ", marks.count, format_size(marks.total_bytes))
            } else {
                format!(" ✦ {} ", marks.count)
            };
            spans.push(Span::styled(
                "│",
                Style::default()
                    .fg(palette.text_muted)
                    .bg(palette.status_bg),
            ));
            spans.push(Span::styled(
                size_str,
                Style::default()
                    .fg(palette.accent_yellow)
                    .bg(palette.status_bg),
            ));
        }
        spans.push(Span::styled(
            "│",
            Style::default()
                .fg(palette.text_muted)
                .bg(palette.status_bg),
        ));
        spans.push(Span::styled(
            zones.workspace.clone(),
            Style::default()
                .fg(palette.accent_mauve)
                .bg(palette.status_workspace_bg)
                .add_modifier(Modifier::BOLD),
        ));
        // Clock is rendered right-aligned in a separate area below.
    }

    if zones.progress.is_some() {
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
    } else {
        let clock_text = format!(" {} ", zones.clock);
        let right_width = (clock_text.chars().count() + 1) as u16; // +1 for │ divider
        let [body_area, clock_area] = Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(right_width),
        ])
        .areas(area);
        frame.render_widget(Paragraph::new(Line::from(spans)), body_area);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    "│",
                    Style::default()
                        .fg(palette.text_muted)
                        .bg(palette.status_bg),
                ),
                Span::styled(
                    clock_text,
                    Style::default()
                        .fg(palette.accent_mauve)
                        .bg(palette.status_bg)
                        .add_modifier(Modifier::BOLD),
                ),
            ])),
            clock_area,
        );
    }
}

fn format_size(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.0} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn render_key_hints(
    frame: &mut Frame<'_>,
    area: Rect,
    state: &AppState,
    palette: crate::config::ThemePalette,
) {
    use crate::state::ModalKind;

    let hints: &[(&str, &str)] = match state.focus_layer() {
        crate::state::FocusLayer::Modal(ModalKind::Dialog) => &[
            ("\u{2191}\u{2193}", "Scroll"),
            ("PgUp/Dn", "Page"),
            ("Esc", "Close"),
        ],
        crate::state::FocusLayer::Modal(ModalKind::Collision) => &[
            ("O", "Overwrite"),
            ("R", "Rename"),
            ("S", "Skip"),
            ("Esc", "Cancel"),
        ],
        crate::state::FocusLayer::Modal(ModalKind::Prompt) => {
            &[("Enter", "Confirm"), ("Esc", "Cancel")]
        }
        crate::state::FocusLayer::Modal(ModalKind::Settings) => &[
            ("\u{2191}\u{2193}", "Navigate"),
            ("Space", "Toggle"),
            ("Esc", "Close"),
        ],
        crate::state::FocusLayer::Modal(ModalKind::Bookmarks) => {
            &[("Enter", "Go"), ("Del", "Remove"), ("Esc", "Close")]
        }
        crate::state::FocusLayer::Modal(ModalKind::Palette)
        | crate::state::FocusLayer::Modal(ModalKind::FileFinder) => &[
            ("\u{2191}\u{2193}", "Navigate"),
            ("Enter", "Open"),
            ("Esc", "Cancel"),
        ],
        crate::state::FocusLayer::Editor => &[
            ("Ctrl+S", "Save"),
            ("Ctrl+F", "Find"),
            ("F3", "Next"),
            ("Esc", "Close"),
        ],
        crate::state::FocusLayer::Preview | crate::state::FocusLayer::MarkdownPreview => {
            &[("Ctrl+W", "Cycle"), ("PgUp/Dn", "Scroll"), ("Esc", "Close")]
        }
        _ => &[
            ("Alt+1..4", "Workspace"),
            ("F1", "Help"),
            ("F3", "View"),
            ("F4", "Edit"),
            ("F5", "Copy"),
            ("F6", "Rename"),
            ("F7", "Mkdir"),
            ("F8", "Delete"),
            ("F10", "Quit"),
        ],
    };

    let key_style = Style::default()
        .fg(palette.surface_bg)
        .bg(palette.key_hint_fg)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default()
        .fg(palette.text_primary)
        .bg(palette.surface_bg);
    let sep_style = Style::default().bg(palette.surface_bg);

    let mut spans: Vec<Span> = Vec::new();
    let mut used_width = 0u16;

    for (key, label) in hints {
        let key_text = format!(" {} ", key);
        let label_text = format!(" {} ", label);
        let segment_width = (key_text.chars().count() + label_text.chars().count()) as u16;
        if used_width + segment_width > area.width {
            break;
        }
        spans.push(Span::styled(key_text, key_style));
        spans.push(Span::styled(label_text, label_style));
        used_width += segment_width;
    }

    // Fill remainder with status background so the bar doesn't look torn.
    if used_width < area.width {
        spans.push(Span::styled(
            " ".repeat((area.width - used_width) as usize),
            sep_style,
        ));
    }

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

#[cfg(test)]
mod tests {
    use crate::config::{IconMode, ThemePalette};
    use crate::editor::EditorBuffer;
    use crate::fs::EntryKind;
    use crate::icon::icon_for_kind;
    use crate::palette::all_entries;
    use crate::preview::ViewBuffer;
    use ratatui::layout::Rect;
    use ratatui::style::{Color, Modifier};

    use super::editor::editor_render_state;
    use super::menu_bar::{top_bar_logo_spans, workspace_switcher_spans};
    use super::pane::{format_icon_slot, pane_chrome_style};
    use super::styles::{
        command_palette_entry_hint_style, command_palette_entry_label_style,
        command_palette_header_style, elevated_surface_style, overlay_title_style,
    };

    fn test_palette() -> ThemePalette {
        ThemePalette {
            menu_bg: Color::Rgb(10, 11, 12),
            menu_fg: Color::Rgb(20, 21, 22),
            menu_active_bg: Color::Rgb(30, 31, 32),
            menu_mnemonic_fg: Color::Rgb(40, 41, 42),
            border_focus: Color::Rgb(50, 51, 52),
            border_editor_focus: Color::Rgb(60, 61, 62),
            selection_bg: Color::Rgb(70, 71, 72),
            selection_fg: Color::Rgb(80, 81, 82),
            surface_bg: Color::Rgb(90, 91, 92),
            tools_bg: Color::Rgb(100, 101, 102),
            prompt_bg: Color::Rgb(110, 111, 112),
            prompt_border: Color::Rgb(120, 121, 122),
            text_primary: Color::Rgb(130, 131, 132),
            text_muted: Color::Rgb(140, 141, 142),
            directory_fg: Color::Rgb(150, 151, 152),
            symlink_fg: Color::Rgb(160, 161, 162),
            file_fg: Color::Rgb(170, 171, 172),
            status_bg: Color::Rgb(180, 181, 182),
            status_fg: Color::Rgb(190, 191, 192),
            logo_accent: Color::Rgb(200, 201, 202),
            key_hint_fg: Color::Rgb(210, 211, 212),
            syntect_theme: "test",
            search_match_bg: Color::Rgb(80, 64, 0),
            search_match_active_bg: Color::Rgb(185, 140, 10),
            text_sel_bg: Color::Rgb(35, 85, 145),
            text_subtext: Color::Rgb(220, 221, 222),
            accent_mauve: Color::Rgb(230, 231, 232),
            accent_teal: Color::Rgb(240, 241, 242),
            accent_green: Color::Rgb(250, 251, 252),
            accent_yellow: Color::Rgb(10, 20, 30),
            accent_peach: Color::Rgb(40, 50, 60),
            accent_red: Color::Rgb(70, 80, 90),
            modal_halo: Color::Rgb(100, 110, 120),
            pane_filter_bg: Color::Rgb(130, 140, 150),
            pane_filter_border: Color::Rgb(160, 170, 180),
            status_git_bg: Color::Rgb(190, 200, 210),
            status_entry_bg: Color::Rgb(220, 230, 240),
            status_workspace_bg: Color::Rgb(250, 10, 20),
        }
    }

    #[test]
    fn unicode_icons_use_glyphs() {
        assert_eq!(icon_for_kind(EntryKind::Directory, IconMode::Unicode), "▣");
        assert_eq!(icon_for_kind(EntryKind::File, IconMode::Unicode), "•");
        assert_eq!(icon_for_kind(EntryKind::Symlink, IconMode::Unicode), "↗");
        assert_eq!(icon_for_kind(EntryKind::Other, IconMode::Unicode), "◦");
    }

    #[test]
    fn ascii_icons_use_labels() {
        assert_eq!(icon_for_kind(EntryKind::Directory, IconMode::Ascii), "[D]");
        assert_eq!(icon_for_kind(EntryKind::File, IconMode::Ascii), "[F]");
        assert_eq!(icon_for_kind(EntryKind::Symlink, IconMode::Ascii), "[L]");
        assert_eq!(icon_for_kind(EntryKind::Other, IconMode::Ascii), "[?]");
    }

    #[test]
    fn elevated_surface_uses_tools_bg() {
        let p = test_palette();
        let s = elevated_surface_style(p);
        assert_eq!(s.bg, Some(p.tools_bg));
    }

    #[test]
    fn overlay_title_is_bold_and_mnemonic_fg() {
        let p = test_palette();
        let s = overlay_title_style(p);
        assert_eq!(s.fg, Some(p.menu_mnemonic_fg));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn pane_chrome_focused_uses_border_focus_color() {
        let p = test_palette();
        let chrome = pane_chrome_style(true, p);
        assert_eq!(chrome.border.fg, Some(p.border_focus));
    }

    #[test]
    fn top_bar_logo_has_five_spans() {
        let p = test_palette();
        let spans = top_bar_logo_spans(true, p);
        assert_eq!(spans.len(), 5);
        assert_eq!(spans[2].content, "Z");
    }

    #[test]
    fn top_bar_workspace_indicator_has_four_pills_and_highlights_active() {
        let p = test_palette();
        let spans = workspace_switcher_spans(2, 4, None, true, p);

        let labels = spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<Vec<_>>();
        assert!(labels.contains(&" 1 "));
        assert!(labels.contains(&" 2 "));
        // workspace 3 is active (index 2), has CWD prefix " 3 "
        assert!(labels.iter().any(|l| l.contains('3')));
        assert!(labels.contains(&" 4 "));

        let active = spans
            .iter()
            .find(|span| span.content.contains('3'))
            .expect("active workspace pill should exist");
        let inactive = spans
            .iter()
            .find(|span| span.content.as_ref() == " 1 ")
            .expect("inactive workspace pill should exist");
        assert_eq!(active.style.bg, Some(p.selection_bg));
        assert_eq!(active.style.fg, Some(p.selection_fg));
        assert!(active.style.add_modifier.contains(Modifier::REVERSED));
        assert!(active.style.add_modifier.contains(Modifier::UNDERLINED));
        assert_eq!(inactive.style.bg, Some(p.menu_active_bg));
        assert!(!inactive.style.add_modifier.contains(Modifier::REVERSED));
    }

    #[test]
    fn command_palette_selected_entry_uses_selection_bg() {
        let p = test_palette();
        let s = command_palette_entry_label_style(true, p);
        assert_eq!(s.bg, Some(p.selection_bg));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn command_palette_unselected_entry_uses_text_primary() {
        let p = test_palette();
        let s = command_palette_entry_label_style(false, p);
        assert_eq!(s.fg, Some(p.text_primary));
    }

    #[test]
    fn command_palette_header_is_muted_and_bold() {
        let p = test_palette();
        let s = command_palette_header_style(p);
        assert_eq!(s.fg, Some(p.text_muted));
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn command_palette_hint_uses_key_hint_fg() {
        let p = test_palette();
        let s = command_palette_entry_hint_style(p);
        assert_eq!(s.fg, Some(p.key_hint_fg));
    }

    #[test]
    fn icon_slot_unicode_appends_two_spaces() {
        let slot = format_icon_slot("▣", IconMode::Unicode);
        assert_eq!(slot, "▣  ");
    }

    #[test]
    fn icon_slot_ascii_returns_icon_only() {
        let slot = format_icon_slot("[D]", IconMode::Ascii);
        assert_eq!(slot, "[D]");
    }

    #[test]
    fn all_palette_entries_have_non_empty_labels() {
        for entry in all_entries() {
            assert!(!entry.label.is_empty(), "entry label is empty: {:?}", entry);
        }
    }

    #[test]
    fn editor_render_state_tracks_viewport() {
        let mut editor = EditorBuffer::default();
        let area = Rect {
            x: 0,
            y: 0,
            width: 80,
            height: 24,
        };
        let rs = editor_render_state(&mut editor, area, true, 4, false);
        assert_eq!(rs.visible_start, 0);
    }

    #[test]
    fn view_buffer_visible_window_returns_correct_slice() {
        let vb = ViewBuffer::from_plain("line1\nline2\nline3");
        let (first, window) = vb.visible_window(2);
        assert_eq!(first, 0);
        assert_eq!(window.len(), 2);
    }
}

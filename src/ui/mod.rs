mod bookmarks;
mod code_view;
mod editor;
mod finder;
pub mod markdown;
mod menu_bar;
mod overlay;
mod palette;
mod pane;
pub mod preview;
mod settings;
pub mod ssh;
pub mod terminal;
mod styles;

pub mod layout_cache;
pub use layout_cache::LayoutCache;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::pane::PaneId;
use crate::state::{AppState, PaneLayout};
use crate::ui::bookmarks::render_bookmarks_modal;
use crate::ui::editor::{editor_render_state, render_editor, RenderEditorArgs};
use crate::ui::finder::render_file_finder;
use crate::ui::markdown::render_markdown_preview;
use crate::ui::menu_bar::render_menu_bar;
use crate::ui::overlay::{
    menu_popup_width, render_collision_dialog, render_dialog, render_menu_popup, render_prompt,
};
use crate::ui::palette::render_command_palette;
use crate::ui::pane::{render_pane, RenderPaneArgs};
use crate::ui::preview::render_preview_panel;
use crate::ui::settings::render_settings_panel;
use crate::ui::ssh::render_ssh_connect_dialog;

use ratatui::widgets::Borders;

/// Render the full TUI. Returns a `LayoutCache` recording each panel's `Rect`
/// so the event loop can use it for mouse hit-testing without re-running layout.
pub fn render(frame: &mut Frame<'_>, state: &mut AppState) -> LayoutCache {
    let mut cache = LayoutCache::default();
    let palette = state.theme().palette;
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
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
        let splits = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Percentage(30)])
            .split(areas[1]);
        (splits[0], Some(splits[1]))
    } else {
        (areas[1], None)
    };

    if let Some(t_area) = terminal_area {
        cache.terminal_panel = Some(t_area);
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
        let panes = Layout::default()
            .direction(pane_direction)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
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

            if let Some(editor) = state.editor_mut() {
                let editor_view = editor_render_state(editor, editor_area, editor_focused);
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
                        render_markdown_preview(
                            frame, md_area, &source, palette, md_scroll, md_focused,
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
                preview_view,
                &filename,
                state.is_preview_focused(),
                palette,
                cheap_tools_mode && !state.is_preview_focused(),
            );
        }
    }

    let mut menu_popup_rect: Option<Rect> = None;
    if let Some(menu) = state.active_menu() {
        let item_count = state.menu_items().len();
        let editor_menu_mode = state.is_editor_fullscreen() && state.editor().is_some();
        let mut popup_x = areas[0].x + 1;
        let mut cursor = areas[0].x + 8;
        for tab in crate::state::menu_tabs(editor_menu_mode) {
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
            editor_menu_mode,
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

    let status = Paragraph::new(Line::raw(state.status_line())).style(
        Style::default()
            .fg(palette.status_fg)
            .bg(palette.status_bg)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(status, areas[2]);

    LayoutCache {
        menu_bar: areas[0],
        left_pane: left_pane_rect,
        right_pane: right_pane_rect,
        tools_panel: tools_area_opt,
        editor_panel: editor_panel_rect,
        file_preview_panel: file_preview_panel_rect,
        markdown_preview_panel: markdown_preview_panel_rect,
        status_bar: areas[2],
        menu_popup: menu_popup_rect,
        terminal_panel: terminal_area,
    }
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
    use super::menu_bar::top_bar_logo_spans;
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
        let rs = editor_render_state(&mut editor, area, true);
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

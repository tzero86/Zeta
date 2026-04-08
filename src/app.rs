use std::io::{self, Stdout};
use std::time::Duration;
use std::time::Instant;

use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, TryRecvError};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::
    {disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};

use crate::action::{Action, Command};
use crate::config::{AppConfig, RuntimeKeymap};
use crate::editor::EditorBuffer;
use crate::event::AppEvent;
use crate::jobs::{self, FileOpRequest, JobResult, PreviewRequest, ScanRequest, WorkerChannels};
use crate::state::{AppState, FocusLayer, ModalKind};
use crate::ui;
use crate::ui::layout_cache::{rect_contains, LayoutCache};

type TuiTerminal = Terminal<CrosstermBackend<Stdout>>;

pub struct App {
    workers: WorkerChannels,
    job_results: Receiver<JobResult>,
    keymap: RuntimeKeymap,
    state: AppState,
    pub layout_cache: LayoutCache,
}

impl App {
    pub fn bootstrap() -> Result<Self> {
        let started_at = Instant::now();
        let loaded_config =
            AppConfig::load_default_location().context("failed to resolve application config")?;
        let keymap = loaded_config
            .config
            .compile_keymap()
            .context("failed to compile configured key bindings")?;
        let (workers, job_results) = jobs::spawn_workers();
        let state = AppState::bootstrap(loaded_config, started_at)
            .context("failed to bootstrap application state")?;
        let mut app = Self {
            workers,
            job_results,
            keymap,
            state,
            layout_cache: LayoutCache::default(),
        };

        for command in app.state.initial_commands() {
            app.execute_command(command)?;
        }

        Ok(app)
    }

    pub fn run(&mut self) -> Result<()> {
        let mut terminal = TerminalSession::enter()?;

        while !self.state.should_quit() {
            let mut cache = LayoutCache::default();
            terminal.draw(|frame| {
                cache = ui::render(frame, &mut self.state);
            })?;
            self.layout_cache = cache;
            self.state.mark_drawn();
            self.process_next_event()?;
        }

        Ok(())
    }

    fn process_next_event(&mut self) -> Result<()> {
        let Some(app_event) = self.next_event()? else {
            return Ok(());
        };

        self.handle_event(app_event)?;

        Ok(())
    }

    fn next_event(&mut self) -> Result<Option<AppEvent>> {
        match self.job_results.try_recv() {
            Ok(result) => return Ok(Some(AppEvent::Job(result))),
            Err(TryRecvError::Disconnected) => {
                anyhow::bail!("background worker disconnected")
            }
            Err(TryRecvError::Empty) => {}
        }

        if !event::poll(Duration::from_millis(250)).context("failed to poll terminal events")? {
            return Ok(None);
        }

        match event::read().context("failed to read terminal event")? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                Ok(Some(AppEvent::Input(key_event)))
            }
            Event::Mouse(mouse_event) => Ok(Some(AppEvent::Mouse(mouse_event))),
        Event::Resize(width, height) => Ok(Some(AppEvent::Resize { width, height })),
            _ => Ok(None),
        }
    }

    fn handle_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Input(key_event) => {
                let focus = self.state.focus_layer();
                let is_preview_open = self.state.is_preview_panel_open();
                if let Some(action) =
                    route_key_event(key_event, &self.keymap, focus, is_preview_open)
                {
                    self.dispatch(action)?;
                }
            }
            AppEvent::Mouse(mouse_event) => {
                let focus = self.state.focus_layer();
                if let Some(action) =
                    route_mouse_event(mouse_event, &self.layout_cache, focus)
                {
                    self.dispatch(action)?;
                }
            }
            AppEvent::Resize { width, height } => {
                self.dispatch(Action::Resize { width, height })?;
            }
            AppEvent::Job(result) => {
                self.state.apply_job_result(result);
            }
        }
        Ok(())
    }

    fn dispatch(&mut self, action: Action) -> Result<()> {
        for command in self.state.apply(action)? {
            self.execute_command(command)?;
        }

        Ok(())
    }

    fn execute_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::OpenEditor { path } => match EditorBuffer::open(&path) {
                Ok(editor) => self.state.open_editor(editor),
                Err(error) => self
                    .state
                    .set_error_status(format!("failed to open editor buffer: {error}")),
            },
            Command::PreviewFile { path } => self
                .workers
                .preview_tx
                .send(PreviewRequest {
                    path,
                    syntect_theme: self.state.theme().palette.syntect_theme.to_string(),
                })
                .context("failed to queue background preview job")?,
            Command::RunFileOperation {
                operation,
                refresh,
                collision,
            } => self
                .workers
                .file_op_tx
                .send(FileOpRequest { operation, refresh, collision })
                .context("failed to queue background file operation")?,
            Command::ScanPane { pane, path } => self
                .workers
                .scan_tx
                .send(ScanRequest { pane, path })
                .context("failed to queue background scan job")?,
            Command::DispatchAction(action) => {
                self.dispatch(action)?;
            }
            Command::SaveEditor => {
                if let Some(editor) = self.state.editor_mut() {
                    match editor.save() {
                        Ok(()) => self.state.mark_editor_saved(),
                        Err(error) => self
                            .state
                            .set_error_status(format!("failed to save editor buffer: {error}")),
                    }
                } else {
                    self.state.set_error_status("no editor buffer is open");
                }
            }
        }

        Ok(())
    }
}

fn route_key_event(
    key_event: crossterm::event::KeyEvent,
    keymap: &RuntimeKeymap,
    focus: FocusLayer,
    is_preview_open: bool,
) -> Option<Action> {
    use crossterm::event::{KeyCode, KeyModifiers};

    let alt_f3 = key_event.code == KeyCode::F(3)
        && key_event.modifiers == KeyModifiers::ALT;

    match focus {
        FocusLayer::Modal(ModalKind::Palette) => Action::from_palette_key_event(key_event),
        FocusLayer::Modal(ModalKind::Collision) => Action::from_collision_key_event(key_event),
        FocusLayer::Modal(ModalKind::Prompt) => Action::from_prompt_key_event(key_event),
        FocusLayer::Modal(ModalKind::Dialog) => Action::from_dialog_key_event(key_event),
        FocusLayer::Modal(ModalKind::Menu) => Action::from_menu_key_event(key_event),
        FocusLayer::Modal(ModalKind::Settings) => Action::from_settings_key_event(key_event),
        FocusLayer::Preview => Action::from_preview_key_event(key_event),
        FocusLayer::Editor => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_editor_key_event(key_event)
                .or_else(|| Action::from_pane_key_event(key_event, keymap))
        }
        FocusLayer::Pane => {
            if is_preview_open && alt_f3 {
                return Some(Action::FocusPreviewPanel);
            }
            Action::from_pane_key_event(key_event, keymap)
        }
    }
}

/// Translate a raw mouse event into an `Action` using the last-rendered
/// `LayoutCache` for hit-testing. Returns `None` for unhandled events.
fn route_mouse_event(
    event: crossterm::event::MouseEvent,
    cache: &LayoutCache,
    focus: FocusLayer,
) -> Option<Action> {
    use crossterm::event::{MouseButton, MouseEventKind};

    let col = event.column;
    let row = event.row;

    match event.kind {
        // -------------------------------------------------------------------
        // Scroll wheel
        // -------------------------------------------------------------------
        MouseEventKind::ScrollUp => {
            if focus == FocusLayer::Preview
                || cache.tools_panel.is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollPreviewUp);
            }
            if focus == FocusLayer::Editor {
                return Some(Action::EditorMoveUp);
            }
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionUp);
            }
            None
        }
        MouseEventKind::ScrollDown => {
            if focus == FocusLayer::Preview
                || cache.tools_panel.is_some_and(|r| rect_contains(r, col, row))
            {
                return Some(Action::ScrollPreviewDown);
            }
            if focus == FocusLayer::Editor {
                return Some(Action::EditorMoveDown);
            }
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                return Some(Action::MoveSelectionDown);
            }
            None
        }

        // -------------------------------------------------------------------
        // Left click
        // -------------------------------------------------------------------
        MouseEventKind::Down(MouseButton::Left) => {
            // When a menu is already open, still allow clicks on the menu bar
            // so the user can switch menus naturally by clicking.
            if matches!(focus, FocusLayer::Modal(ModalKind::Menu))
                && rect_contains(cache.menu_bar, col, row)
            {
                return route_menu_bar_click(col, cache.menu_bar.x);
            }

            // All other modal states absorb left clicks.
            if matches!(focus, FocusLayer::Modal(_)) {
                return None;
            }

            // Click on menu bar item.
            if rect_contains(cache.menu_bar, col, row) {
                return route_menu_bar_click(col, cache.menu_bar.x);
            }

            // Click on the tools panel (editor or preview).
            if let Some(tools_rect) = cache.tools_panel {
                if rect_contains(tools_rect, col, row) {
                    if focus != FocusLayer::Editor {
                        return Some(Action::FocusPreviewPanel);
                    }
                    return None; // editor already focused
                }
            }

            // Click on left or right pane — switch focus if coming from tools.
            if rect_contains(cache.left_pane, col, row)
                || rect_contains(cache.right_pane, col, row)
            {
                if focus == FocusLayer::Editor || focus == FocusLayer::Preview {
                    return Some(Action::CycleFocus);
                }
                return Some(Action::FocusNextPane);
            }

            None
        }

        _ => None,
    }
}

/// Map an x-coordinate in the menu bar to an `OpenMenu` action.
/// Layout (0-indexed from bar_x):
///   0-7   " [Z]eta "  (logo — ignored)
///   8-13  " File "
///   14-23 " Navigate "
///   24-29 " View "
///   30-35 " Help "
fn route_menu_bar_click(col: u16, bar_x: u16) -> Option<Action> {
    use crate::action::MenuId;
    let offset = col.saturating_sub(bar_x);
    match offset {
        8..=13 => Some(Action::OpenMenu(MenuId::File)),
        14..=23 => Some(Action::OpenMenu(MenuId::Navigate)),
        24..=29 => Some(Action::OpenMenu(MenuId::View)),
        30..=35 => Some(Action::OpenMenu(MenuId::Help)),
        _ => None,
    }
}

struct TerminalSession {
    terminal: TuiTerminal,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        enable_raw_mode().context("failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
                .context("failed to enter alternate screen and enable mouse")?;

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).context("failed to create terminal backend")?;
        terminal.clear().context("failed to clear terminal")?;

        Ok(Self { terminal })
    }

    fn draw<F>(&mut self, render: F) -> Result<()>
    where
        F: FnOnce(&mut Frame<'_>),
    {
        self.terminal
            .draw(render)
            .context("failed to render terminal frame")?;
        Ok(())
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ratatui::layout::Rect;

    use crate::action::Action;
    use crate::config::RuntimeKeymap;
    use crate::state::{FocusLayer, ModalKind};
    use crate::ui::layout_cache::LayoutCache;

    use super::{route_key_event, route_mouse_event};

    fn test_cache() -> LayoutCache {
        LayoutCache {
            menu_bar:   Rect { x: 0,  y: 0,  width: 80, height: 1  },
            left_pane:  Rect { x: 0,  y: 1,  width: 40, height: 20 },
            right_pane: Rect { x: 40, y: 1,  width: 40, height: 20 },
            tools_panel: None,
            status_bar: Rect { x: 0,  y: 21, width: 80, height: 1  },
        }
    }

    #[test]
    fn mouse_event_variant_exists_in_app_event() {
        let ev = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5, row: 3,
            modifiers: KeyModifiers::NONE,
        };
        let app_event = crate::event::AppEvent::Mouse(ev);
        assert!(matches!(app_event, crate::event::AppEvent::Mouse(_)));
    }

    #[test]
    fn route_mouse_scroll_up_in_pane_produces_move_selection_up() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollUp, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane,
        );
        assert_eq!(action, Some(Action::MoveSelectionUp));
    }

    #[test]
    fn route_mouse_scroll_down_in_pane_produces_move_selection_down() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollDown, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane,
        );
        assert_eq!(action, Some(Action::MoveSelectionDown));
    }

    #[test]
    fn route_mouse_left_click_on_pane_produces_action() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane,
        );
        assert!(action.is_some(), "expected an action for left-pane click");
    }

    #[test]
    fn route_mouse_left_click_on_file_menu_opens_file_menu() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 10, row: 0, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Pane,
        );
        assert_eq!(action, Some(Action::OpenMenu(crate::action::MenuId::File)));
    }

    #[test]
    fn route_mouse_modal_absorbs_left_click() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Modal(ModalKind::Dialog),
        );
        assert_eq!(action, None);
    }

    #[test]
    fn route_mouse_scroll_in_preview_layer_scrolls_preview() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollDown, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Preview,
        );
        assert_eq!(action, Some(Action::ScrollPreviewDown));
    }

    #[test]
    fn route_mouse_scroll_in_editor_layer_moves_cursor() {
        let action = route_mouse_event(
            MouseEvent { kind: MouseEventKind::ScrollUp, column: 10, row: 5, modifiers: KeyModifiers::NONE },
            &test_cache(), FocusLayer::Editor,
        );
        assert_eq!(action, Some(Action::EditorMoveUp));
    }

    #[test]
    fn command_palette_remains_available_while_editor_is_open() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Editor, // editor focused, palette NOT open
            false,
        );
        assert_eq!(action, Some(Action::OpenCommandPalette));
    }

    #[test]
    fn editor_shortcuts_still_take_priority_over_global_fallbacks() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Editor,
            false,
        );
        assert_eq!(action, Some(Action::EditorOpenSearch));
    }

    #[test]
    fn palette_open_state_blocks_lower_priority_input_paths() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Modal(ModalKind::Palette),
            false,
        );
        assert_eq!(action, None);
    }

    #[test]
    fn palette_layer_routes_esc_to_close_palette() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            &keymap,
            FocusLayer::Modal(ModalKind::Palette),
            false,
        );
        assert_eq!(action, Some(Action::CloseCommandPalette));
    }

    #[test]
    fn pane_layer_ctrl_q_quits() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Pane,
            false,
        );
        assert_eq!(action, Some(Action::Quit));
    }

    #[test]
    fn editor_layer_ctrl_f_opens_search() {
        let keymap = RuntimeKeymap::default();
        let action = route_key_event(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            &keymap,
            FocusLayer::Editor,
            false,
        );
        assert_eq!(action, Some(Action::EditorOpenSearch));
    }
}

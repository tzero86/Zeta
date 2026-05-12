use crate::action::{Action, Command};
use anyhow::Result;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[cfg(feature = "terminal-panel")]
use vt100::Parser as VtParser;

/// Platform-specific parser wrapper. When `terminal-panel` is disabled this
/// is a zero-cost no-op stub so `TerminalState` can remain in the workspace
/// without pulling the `vt100` crate.
pub struct TerminalParser {
    #[cfg(feature = "terminal-panel")]
    inner: Arc<Mutex<VtParser>>,
    #[cfg(not(feature = "terminal-panel"))]
    _dummy: (),
}

impl TerminalParser {
    pub fn new(rows: u16, cols: u16) -> Self {
        #[cfg(feature = "terminal-panel")]
        {
            Self {
                inner: Arc::new(Mutex::new(VtParser::new(rows, cols, 0))),
            }
        }
        #[cfg(not(feature = "terminal-panel"))]
        {
            Self { _dummy: () }
        }
    }

    pub fn reset(&self, rows: u16, cols: u16) {
        #[cfg(feature = "terminal-panel")]
        if let Ok(mut p) = self.inner.lock() {
            *p = VtParser::new(rows, cols, 0);
        }
    }

    pub fn set_size(&self, rows: u16, cols: u16) {
        #[cfg(feature = "terminal-panel")]
        if let Ok(mut p) = self.inner.lock() {
            p.set_size(rows, cols);
        }
    }

    pub fn process(&self, bytes: &[u8]) {
        #[cfg(feature = "terminal-panel")]
        if let Ok(mut p) = self.inner.lock() {
            p.process(bytes);
        }
    }

    #[cfg(feature = "terminal-panel")]
    pub fn lock(&self) -> Option<std::sync::MutexGuard<'_, VtParser>> {
        self.inner.lock().ok()
    }

    #[cfg(not(feature = "terminal-panel"))]
    pub fn lock(&self) -> Option<()> {
        None
    }
}

impl Clone for TerminalParser {
    fn clone(&self) -> Self {
        #[cfg(feature = "terminal-panel")]
        {
            Self {
                inner: Arc::clone(&self.inner),
            }
        }
        #[cfg(not(feature = "terminal-panel"))]
        {
            Self { _dummy: () }
        }
    }
}

pub struct TerminalState {
    pub open: bool,
    pub focused: bool,
    pub spawned: bool,
    pub parser: TerminalParser,
    pub rows: u16,
    pub cols: u16,
    pub bytes_received: u64,
    pub spawn_id: u64,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            open: false,
            focused: false,
            spawned: false,
            parser: TerminalParser::new(24, 80),
            rows: 24,
            cols: 80,
            bytes_received: 0,
            spawn_id: 0,
        }
    }
}

impl fmt::Debug for TerminalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalState")
            .field("open", &self.open)
            .field("focused", &self.focused)
            .field("spawned", &self.spawned)
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .field("bytes_received", &self.bytes_received)
            .finish()
    }
}

impl TerminalState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Close the terminal panel and reset state so a fresh session
    /// can be spawned next time the user toggles the terminal.
    pub fn close(&mut self) {
        self.open = false;
        self.focused = false;
        self.spawned = false;
        self.bytes_received = 0;
        self.parser.reset(self.rows, self.cols);
    }

    pub fn toggle(&mut self, cwd: PathBuf) -> Vec<Command> {
        self.open = !self.open;
        if self.open {
            self.focused = true;
            if !self.spawned {
                self.spawned = true;
                self.bytes_received = 0;
                self.parser.reset(self.rows, self.cols);
                self.spawn_id += 1;
                vec![Command::SpawnTerminal {
                    cwd,
                    spawn_id: self.spawn_id,
                }]
            } else {
                vec![]
            }
        } else {
            self.focused = false;
            vec![]
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) -> Vec<Command> {
        if rows == self.rows && cols == self.cols {
            return vec![];
        }
        self.rows = rows;
        self.cols = cols;
        self.parser.set_size(rows, cols);
        vec![Command::ResizeTerminal { cols, rows }]
    }

    pub fn process_output(&mut self, bytes: &[u8]) {
        self.bytes_received += bytes.len() as u64;
        self.parser.process(bytes);
    }

    pub fn apply(&mut self, action: &Action, cwd: PathBuf) -> Result<Vec<Command>> {
        let mut commands = Vec::new();
        match action {
            Action::ToggleTerminal => {
                commands.extend(self.toggle(cwd));
            }
            Action::TerminalInput(bytes) => {
                commands.push(Command::WriteTerminal(bytes.clone()));
            }
            _ => {}
        }
        Ok(commands)
    }
}

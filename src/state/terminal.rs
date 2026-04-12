use crate::action::{Action, Command};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fmt;

pub struct TerminalState {
    pub open: bool,
    pub focused: bool,
    pub spawned: bool,
    pub parser: Arc<Mutex<vt100::Parser>>,
    pub rows: u16,
    pub cols: u16,
    pub bytes_received: u64,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            open: false,
            focused: false,
            spawned: false,
            parser: Arc::new(Mutex::new(vt100::Parser::new(24, 80, 0))),
            rows: 24,
            cols: 80,
            bytes_received: 0,
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
        if let Ok(mut parser) = self.parser.lock() {
            *parser = vt100::Parser::new(self.rows, self.cols, 0);
        }
    }

    pub fn toggle(&mut self, cwd: PathBuf) -> Vec<Command> {
        self.open = !self.open;
        if self.open {
            self.focused = true;
            if !self.spawned {
                self.spawned = true;
                self.bytes_received = 0;
                // Clear current screen by creating a new parser
                if let Ok(mut parser) = self.parser.lock() {
                    *parser = vt100::Parser::new(self.rows, self.cols, 0);
                }
                vec![Command::SpawnTerminal { cwd }]
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
        if let Ok(mut parser) = self.parser.lock() {
            parser.set_size(rows, cols);
        }
        vec![Command::ResizeTerminal {
            cols,
            rows,
        }]
    }

    pub fn process_output(&mut self, bytes: &[u8]) {
        self.bytes_received += bytes.len() as u64;
        if let Ok(mut parser) = self.parser.lock() {
            parser.process(bytes);
        }
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

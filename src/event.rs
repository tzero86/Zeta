use crossterm::event::KeyEvent;

use crate::jobs::JobResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppEvent {
    Input(KeyEvent),
    Resize { width: u16, height: u16 },
    Job(JobResult),
}

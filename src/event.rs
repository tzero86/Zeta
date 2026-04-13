use crossterm::event::{KeyEvent, MouseEvent};

use crate::jobs::JobResult;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppEvent {
    Input(KeyEvent),
    Mouse(MouseEvent),
    Resize { width: u16, height: u16 },
    Job(Box<JobResult>),
}

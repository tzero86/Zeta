use crate::action::Action;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneFocus {
    Left,
    Right,
    Preview,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaneLayout {
    #[default]
    SideBySide,
    Stacked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MenuItem {
    pub label: &'static str,
    pub shortcut: &'static str,
    pub mnemonic: char,
    pub action: Action,
}

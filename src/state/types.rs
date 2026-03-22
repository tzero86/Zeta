use crate::action::Action;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneFocus {
    Left,
    Right,
    Preview,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaneLayout {
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

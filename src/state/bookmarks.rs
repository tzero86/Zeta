#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BookmarksState {
    pub selection: usize,
}

impl BookmarksState {
    pub fn new() -> Self {
        Self::default()
    }
}

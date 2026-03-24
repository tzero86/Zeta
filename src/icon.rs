use crate::config::IconMode;
use crate::fs::EntryKind;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconKind {
    Directory,
    File,
    Symlink,
    Other,
}

impl From<EntryKind> for IconKind {
    fn from(value: EntryKind) -> Self {
        match value {
            EntryKind::Directory => Self::Directory,
            EntryKind::File => Self::File,
            EntryKind::Symlink => Self::Symlink,
            EntryKind::Other => Self::Other,
        }
    }
}

pub fn icon_for_kind(kind: EntryKind, mode: IconMode) -> &'static str {
    match mode {
        IconMode::Unicode => unicode_icon(kind),
        IconMode::Ascii => kind.ascii_label(),
        IconMode::Custom => custom_icon(kind),
    }
}

fn unicode_icon(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Directory => "▣",
        EntryKind::File => "•",
        EntryKind::Symlink => "↗",
        EntryKind::Other => "◦",
    }
}

fn custom_icon(kind: EntryKind) -> &'static str {
    match IconKind::from(kind) {
        IconKind::Directory => "\u{e001}",
        IconKind::File => "\u{e002}",
        IconKind::Symlink => "\u{e003}",
        IconKind::Other => "\u{e004}",
    }
}

#[cfg(test)]
mod tests {
    use super::icon_for_kind;
    use crate::config::IconMode;
    use crate::fs::EntryKind;

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
    fn custom_icons_use_private_use_glyphs() {
        assert_eq!(
            icon_for_kind(EntryKind::Directory, IconMode::Custom),
            "\u{e001}"
        );
        assert_eq!(icon_for_kind(EntryKind::File, IconMode::Custom), "\u{e002}");
        assert_eq!(
            icon_for_kind(EntryKind::Symlink, IconMode::Custom),
            "\u{e003}"
        );
        assert_eq!(
            icon_for_kind(EntryKind::Other, IconMode::Custom),
            "\u{e004}"
        );
    }
}

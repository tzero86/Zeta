use crate::config::IconMode;
use crate::fs::EntryKind;

pub fn icon_for_kind(kind: EntryKind, mode: IconMode) -> &'static str {
    icon_for_entry(kind, None, mode)
}

pub fn icon_for_entry(kind: EntryKind, extension: Option<&str>, mode: IconMode) -> &'static str {
    match mode {
        IconMode::Unicode => unicode_icon(kind),
        IconMode::Ascii => kind.ascii_label(),
        IconMode::NerdFont => nerdfont_icon(kind, extension),
    }
}

fn unicode_icon(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Directory => "▣",
        EntryKind::File => "•",
        EntryKind::Symlink => "↗",
        EntryKind::Archive => "🗜",
        EntryKind::Other => "◦",
    }
}

fn nerdfont_icon(kind: EntryKind, extension: Option<&str>) -> &'static str {
    match kind {
        EntryKind::Directory => "\u{f07b}",
        EntryKind::Symlink => "\u{f481}",
        EntryKind::Archive => "\u{f410}",
        EntryKind::Other => "\u{f128}",
        EntryKind::File => {
            match extension.map(|e| e.to_ascii_lowercase()).as_deref() {
                Some("rs") => "\u{e7a8}",
                Some("toml") | Some("yaml") | Some("yml") | Some("json") => "\u{e615}",
                Some("md") | Some("mdx") => "\u{f48a}",
                Some("sh") | Some("bash") | Some("zsh") | Some("fish") => "\u{f489}",
                Some("py") => "\u{e606}",
                Some("js") | Some("ts") | Some("jsx") | Some("tsx") => "\u{e74e}",
                Some("go") => "\u{e626}",
                Some("c") | Some("cpp") | Some("h") | Some("hpp") => "\u{e61e}",
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") => "\u{f1c5}",
                Some("zip") | Some("tar") | Some("gz") | Some("bz2") | Some("xz") | Some("7z") => "\u{f410}",
                Some("lock") => "\u{f023}",
                _ => "\u{f15b}",
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{icon_for_entry, icon_for_kind};
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
    fn nerdfont_directory_icon() {
        assert_eq!(
            icon_for_kind(EntryKind::Directory, IconMode::NerdFont),
            "\u{f07b}"
        );
    }

    #[test]
    fn nerdfont_rust_extension() {
        assert_eq!(
            icon_for_entry(EntryKind::File, Some("rs"), IconMode::NerdFont),
            "\u{e7a8}"
        );
    }

    #[test]
    fn nerdfont_generic_file_no_extension() {
        assert_eq!(
            icon_for_entry(EntryKind::File, None, IconMode::NerdFont),
            "\u{f15b}"
        );
    }

    #[test]
    fn nerdfont_image_extension() {
        assert_eq!(
            icon_for_entry(EntryKind::File, Some("png"), IconMode::NerdFont),
            "\u{f1c5}"
        );
    }
}

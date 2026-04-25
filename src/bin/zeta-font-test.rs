//! Diagnostic: prints NerdFont glyphs so the user can verify their terminal font
//! actually contains them. Run with: `cargo run --bin zeta-font-test`
//!
//! For each glyph we print:
//!   - The codepoint label
//!   - The glyph alone, surrounded by `|` markers showing column position
//!   - The glyph followed by 1 space, then a marker
//!   - The glyph followed by 2 spaces, then a marker
//!
//! If your terminal has NerdFont configured:
//!   - You will see actual icons (folder, file, lock, etc.) inside the `|...|`
//!   - The marker positions tell us if the glyph rendered as 1 or 2 columns wide
//!
//! If your terminal does NOT have NerdFont configured:
//!   - You will see blank space, "?" boxes, or tofu characters
//!   - Marker positions will be inconsistent / off-by-one

use unicode_width::UnicodeWidthStr;

fn main() {
    println!("=== Zeta NerdFont diagnostic ===");
    println!();
    println!("If you see actual icons (folder, file, lock, code) below,");
    println!("your terminal font supports NerdFont and Zeta should work.");
    println!();
    println!("Layout: |GLYPH|     |GLYPH |    |GLYPH  |");
    println!("        ^^^^^^^     ^^^^^^^    ^^^^^^^^");
    println!("        bare        +1 space   +2 space");
    println!();
    println!("    unicode-width says 1 column. If your terminal renders the");
    println!("    glyph as 2 columns wide, the right `|` in 'bare' will be");
    println!("    overwritten or shifted.");
    println!();

    let glyphs = [
        ("\u{f07b}", "f07b folder"),
        ("\u{f15b}", "f15b generic file"),
        ("\u{e7a8}", "e7a8 rust"),
        ("\u{e615}", "e615 toml/json"),
        ("\u{f48a}", "f48a markdown"),
        ("\u{f023}", "f023 lock"),
        ("\u{f1c5}", "f1c5 image"),
        ("\u{f410}", "f410 archive"),
    ];

    for (glyph, label) in glyphs {
        let codepoint = glyph.chars().next().unwrap() as u32;
        let unicode_width = UnicodeWidthStr::width(glyph);
        println!(
            "  U+{codepoint:04X} ({label:20}) |{glyph}| |{glyph} | |{glyph}  |    \
             unicode-width={unicode_width}"
        );
    }

    println!();
    println!("=== Reference (known-good Unicode chars) ===");
    println!();
    println!("  These should always render correctly for comparison:");
    let refs = [
        ("\u{2588}", "█ FULL BLOCK (width 1)"),
        ("\u{2580}", "▀ UPPER HALF (width 1)"),
        ("\u{251c}", "├ TREE (width 1)"),
        ("\u{2502}", "│ TREE VERTICAL (width 1)"),
        ("中", "CJK (width 2)"),
    ];
    for (glyph, label) in refs {
        let unicode_width = UnicodeWidthStr::width(glyph);
        println!("  |{glyph}| |{glyph} | |{glyph}  |   {label}  unicode-width={unicode_width}");
    }

    println!();
    println!("=== Verdict ===");
    println!();
    println!(" - If NerdFont glyphs above show as ICONS → terminal font is correct,");
    println!("   the bug is in Zeta's rendering layer.");
    println!(" - If NerdFont glyphs above show as BLANK / boxes / tofu → terminal");
    println!("   font is NOT a NerdFont. Configure your terminal (Warp / Windows");
    println!("   Terminal / WezTerm) to use 'MesloLGS NF', 'FiraCode Nerd Font',");
    println!("   'JetBrainsMono Nerd Font', or similar.");
    println!();
}

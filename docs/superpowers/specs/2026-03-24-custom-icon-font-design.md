# Custom Icon Font Design

## Goal

Add a small custom icon system for Zeta so the app can show branded folder/file/status glyphs without depending only on generic Unicode symbols.

## Problem

Today Zeta uses Unicode or ASCII labels for file icons. That works everywhere, but it limits visual identity and makes the UI feel less distinct.

Terminal apps cannot install or select fonts for the user, so this feature must be treated as an opt-in companion asset, not an automatic replacement.

## Constraints

- Zeta cannot control the terminal emulator font directly.
- Any custom glyphs still depend on the user installing and selecting the font in their terminal.
- The current Unicode and ASCII fallbacks must remain available.
- Keep the implementation lightweight and easy to disable.

## Proposed Design

### 1. Icon set abstraction

Extend `IconMode` so it remains the single source of truth.

- `IconMode::Unicode` keeps the existing symbols.
- `IconMode::Ascii` keeps the existing text labels.
- `IconMode::Custom` uses private-use glyphs from a bundled icon font.

The app never installs or selects terminal fonts. `IconMode::Custom` only changes which glyphs Zeta emits; the user still has to select `assets/fonts/zeta-icons.ttf` in their terminal emulator.

Rendering code should ask for an icon by semantic kind, such as `folder`, `file`, `symlink`, `binary`, `warning`, instead of hardcoded glyph strings.

### 2. Font packaging

Ship a small TTF/OTF asset with the release.

- Store the font at `assets/fonts/zeta-icons.ttf` in the repository and include it in release artifacts.
- Document the font license and keep it compatible with redistribution.
- Use private-use Unicode codepoints for the Zeta icon set.
- Keep the glyph set minimal: folder, file, symlink, executable, config, binary, warning.

### 3. Selection and fallback

Add a config option for the icon source.

- Default behavior remains Unicode.
- If the user sets `icon_mode = "custom"`, Zeta uses the custom glyph map.
- If the custom font asset is missing, Zeta falls back to Unicode, then ASCII.

Do not attempt runtime glyph-support detection; the terminal controls font rendering and the app cannot verify it reliably.

Auto-detection is intentionally conservative: prefer the custom set only when explicitly enabled and the font asset is present.

### 4. Docs and setup

Document the font as an optional enhancement.

- Explain how to install the font in common terminal emulators.
- Explain how to enable custom icons in `config.toml`.
- Keep the default experience working with no extra setup.

## Error Handling

- Missing font asset: fall back to Unicode and continue.
- Invalid icon mode in config: reject during config parsing.
- Unsupported glyph display in terminal: keep rendering the fallback symbols.

## Testing

- Unit tests for icon selection by semantic kind.
- Unit tests for fallback order: custom -> Unicode -> ASCII.
- Config tests for parsing the new icon mode.
- Render tests only with deterministic fixtures; avoid environment-dependent font assertions.

## Non-Goals

- Forcing terminal font installation from inside the app.
- Building a full font editor or theme designer.
- Replacing all text with iconography.

## Success Criteria

- Users can opt into a branded icon set.
- Zeta still works normally on terminals without the font.
- The code path stays simple enough to keep low-overhead TUI goals intact.

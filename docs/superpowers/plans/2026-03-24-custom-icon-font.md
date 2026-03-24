# Custom Icon Font Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional custom icon mode for Zeta while keeping Unicode and ASCII fallbacks working everywhere.

**Architecture:** Centralize icon selection behind a small icon module, extend config with a `custom` mode, and keep rendering code dumb. The app ships a font asset and documentation, but the terminal still owns font selection; Zeta only chooses which glyphs to emit.

**Tech Stack:** Rust, `serde`/`toml`, `ratatui`, existing config/state/UI modules, bundled TTF/OTF asset, unit and render tests.

---

### Task 1: Add icon-mode plumbing and semantic icon mapping

**Files:**
- Modify: `src/config.rs`
- Create: `src/icon.rs`
- Modify: `src/lib.rs` if a new module needs exporting
- Test: `src/icon.rs` unit tests

- [ ] **Step 1: Write the failing test**

Add tests that prove `IconMode` can represent `Custom` and that semantic icon keys resolve to the expected glyph source for `Custom`, `Unicode`, and `Ascii`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test icon::tests -- --exact --nocapture`
Expected: fail because the new mode/module does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Implement a small `IconKind`/`IconSet` helper that returns a string for file, folder, symlink, executable, config, binary, and warning icons. Extend `IconMode` with `Custom` and keep fallback behavior explicit.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test icon::tests -- --exact --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/icon.rs src/lib.rs
git commit -m "feat: add custom icon mode plumbing"
```

### Task 2: Wire UI rendering to the icon module

**Files:**
- Modify: `src/ui.rs`
- Modify: `src/state/dialog.rs` if help/about text needs icon-mode wording
- Test: `src/ui.rs` render/unit tests

- [ ] **Step 1: Write the failing test**

Add a render-focused test that asserts the UI uses the icon helper instead of hardcoded Unicode glyph strings for directory/file rows.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ui::tests::unicode_icons_use_glyphs -- --exact --nocapture`
Expected: fail until the UI reads icons from the new module.

- [ ] **Step 3: Write minimal implementation**

Replace `get_entry_icon` logic with calls into the icon module and thread `IconMode` through any remaining call sites that still hardcode glyphs.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ui::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ui.rs src/state/dialog.rs
git commit -m "feat: route file icons through icon set"
```

### Task 3: Add bundled font asset and release documentation

**Files:**
- Create: `assets/fonts/zeta-icons.ttf`
- Modify: `README.md`
- Modify: any release or packaging notes under `docs/`

- [ ] **Step 1: Write the failing test**

Add a repository-level check that confirms the icon font asset exists at `assets/fonts/zeta-icons.ttf` and is referenced by release notes/docs.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test font_asset -- --nocapture`
Expected: fail until the asset is added.

- [ ] **Step 3: Write minimal implementation**

Add the font file to the repository, ensure it is included in release packaging, and document how users install/select it in their terminal emulator.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test font_asset -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add assets/fonts/zeta-icons.ttf README.md docs/
git commit -m "docs: add custom icon font guidance"
```

### Task 4: Verify fallback behavior and config compatibility

**Files:**
- Modify: `src/config.rs`
- Modify: `src/state/mod.rs` if runtime defaults need adjustment
- Modify: `tests/smoke.rs` or config tests in `src/config.rs`

- [ ] **Step 1: Write the failing test**

Add tests that confirm invalid icon mode values fail config parsing and that missing custom font assets fall back to Unicode/ASCII without crashing.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test config::tests -- --nocapture`
Expected: fail until the new parsing and fallback cases are covered.

- [ ] **Step 3: Write minimal implementation**

Keep `custom` opt-in, preserve existing defaults, and make fallback selection deterministic.

Exact schema: keep the existing `icon_mode` key in `config.toml` and accept `unicode`, `ascii`, or `custom`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test config::tests -- --nocapture`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/config.rs src/state/mod.rs tests/smoke.rs
git commit -m "fix: keep icon font fallback deterministic"
```

### Task 5: Final validation

**Files:**
- All touched files

- [ ] **Step 1: Run formatting and lint checks**

Run: `cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings`

- [ ] **Step 2: Run the full test suite**

Run: `cargo test --workspace`

- [ ] **Step 3: Record results**

If anything fails, fix the smallest failing task first and re-run the relevant test command before proceeding.

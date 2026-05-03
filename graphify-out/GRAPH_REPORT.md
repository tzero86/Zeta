# Graph Report - .  (2026-05-02)

## Corpus Check
- 26 files · ~30,000 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1865 nodes · 3936 edges · 55 communities detected
- Extraction: 77% EXTRACTED · 23% INFERRED · 0% AMBIGUOUS · INFERRED: 900 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_App State & File Ops|App State & File Ops]]
- [[_COMMUNITY_Editor Buffer & Cursor|Editor Buffer & Cursor]]
- [[_COMMUNITY_Filesystem Backend|Filesystem Backend]]
- [[_COMMUNITY_Actions & Keybindings|Actions & Keybindings]]
- [[_COMMUNITY_UI Menus & Overlays|UI Menus & Overlays]]
- [[_COMMUNITY_Archive Worker|Archive Worker]]
- [[_COMMUNITY_Config & Theme|Config & Theme]]
- [[_COMMUNITY_App Entry & Release Scripts|App Entry & Release Scripts]]
- [[_COMMUNITY_Pane Navigation|Pane Navigation]]
- [[_COMMUNITY_Docs & Changelog|Docs & Changelog]]
- [[_COMMUNITY_Overlay & Modal State|Overlay & Modal State]]
- [[_COMMUNITY_Preview System|Preview System]]
- [[_COMMUNITY_Git Integration|Git Integration]]
- [[_COMMUNITY_Icon System|Icon System]]
- [[_COMMUNITY_Markdown Renderer|Markdown Renderer]]
- [[_COMMUNITY_Tech Stack & Conventions|Tech Stack & Conventions]]
- [[_COMMUNITY_SSH & Remote FS|SSH & Remote FS]]
- [[_COMMUNITY_Update & State Types|Update & State Types]]
- [[_COMMUNITY_Hooks & Config Gen|Hooks & Config Gen]]
- [[_COMMUNITY_Release Pipeline|Release Pipeline]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]
- [[_COMMUNITY_Community 29|Community 29]]
- [[_COMMUNITY_Community 30|Community 30]]
- [[_COMMUNITY_Community 31|Community 31]]
- [[_COMMUNITY_Community 32|Community 32]]
- [[_COMMUNITY_Community 33|Community 33]]
- [[_COMMUNITY_Community 34|Community 34]]
- [[_COMMUNITY_Community 35|Community 35]]
- [[_COMMUNITY_Community 36|Community 36]]
- [[_COMMUNITY_Community 37|Community 37]]
- [[_COMMUNITY_Community 38|Community 38]]
- [[_COMMUNITY_Community 39|Community 39]]
- [[_COMMUNITY_Community 40|Community 40]]
- [[_COMMUNITY_Community 41|Community 41]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 58|Community 58]]
- [[_COMMUNITY_Community 59|Community 59]]
- [[_COMMUNITY_Community 60|Community 60]]
- [[_COMMUNITY_Community 62|Community 62]]
- [[_COMMUNITY_Community 63|Community 63]]
- [[_COMMUNITY_Community 64|Community 64]]
- [[_COMMUNITY_Community 65|Community 65]]
- [[_COMMUNITY_Community 66|Community 66]]
- [[_COMMUNITY_Community 67|Community 67]]
- [[_COMMUNITY_Community 68|Community 68]]
- [[_COMMUNITY_Community 69|Community 69]]

## God Nodes (most connected - your core abstractions)
1. `AppState` - 104 edges
2. `test_state()` - 103 edges
3. `ok()` - 66 edges
4. `EditorBuffer` - 49 edges
5. `PaneState` - 44 edges
6. `OverlayState` - 40 edges
7. `parse_markdown_lines()` - 35 edges
8. `render()` - 34 edges
9. `route_key_event()` - 33 edges
10. `Zeta` - 31 edges

## Surprising Connections (you probably didn't know these)
- `Zeta GitHub Pages Site` --semantically_similar_to--> `Site Redesign Design`  [INFERRED] [semantically similar]
  site/index.html → docs/superpowers/specs/2026-04-26-site-redesign-design.md
- `FocusLayer Enum (Input Routing)` --semantically_similar_to--> `Global Command Palette (Ctrl+P)`  [INFERRED] [semantically similar]
  docs/superpowers/plans/2026-04-07-wave2a-input-routing.md → enhancements.md
- `SSH Agent Detection (SSH_AUTH_SOCK)` --semantically_similar_to--> `HostKeyFingerprints (SHA256+MD5)`  [INFERRED] [semantically similar]
  WAVE_7B_PHASE3_IMPLEMENTATION.md → IMPLEMENTATION_SUMMARY.md
- `Site Redesign Design` --references--> `App Screenshot — Norton Commander Style`  [EXTRACTED]
  docs/superpowers/specs/2026-04-26-site-redesign-design.md → screenshot.png
- `Site Redesign Design` --references--> `App Screenshot — Workspaces & Editor`  [EXTRACTED]
  docs/superpowers/specs/2026-04-26-site-redesign-design.md → site/app.png

## Hyperedges (group relationships)
- **Four-Phase Development Roadmap** — phase_1_state_decomp, phase_2_ui_polish, phase_3_first_run, phase_4_shell_hooks [EXTRACTED 1.00]
- **Hook Events with Environment Variables** — on_cd_event, on_open_event, on_start_event, on_exit_event, zeta_path_env, zeta_old_path_env, zeta_pane_env, zeta_version_env [EXTRACTED 1.00]
- **First-Run Wizard Components** — first_run_wizard, theme_picker_step, live_theme_preview, annotated_config_gen, wizard_state [EXTRACTED 1.00]
- **Core Zeta Features** — dual_pane_browser, editor_module, workspaces_feature, ssh_sftp_feature, integrated_terminal, themes_feature, shell_hooks_feature, first_run_wizard [EXTRACTED 0.95]
- **All Completed Development Waves** — roadmap_wave_1a, roadmap_wave_1b, roadmap_wave_1c, roadmap_wave_2a, roadmap_wave_2b, roadmap_wave_3a, roadmap_wave_4a, roadmap_wave_4b, roadmap_wave_4c, roadmap_wave_4d, roadmap_wave_5a, roadmap_wave_5b, roadmap_wave_5c, roadmap_wave_6a, roadmap_wave_6b, roadmap_wave_7a, roadmap_wave_7b, roadmap_wave_8a [EXTRACTED 1.00]
- **Shell Hook Lifecycle Events** — shellhooks_on_start, shellhooks_on_exit, shellhooks_on_cd, shellhooks_on_open [EXTRACTED 1.00]
- **Shell Hook Environment Variables** — shellhooks_zeta_path, shellhooks_zeta_old_path, shellhooks_zeta_pane, shellhooks_zeta_version [EXTRACTED 1.00]
- **Editor-Related Features and Bindings** — roadmap_wave_3a, roadmap_wave_4b, roadmap_wave_4c, roadmap_wave_5a, keybindings_editor [INFERRED 0.80]
- **Git-Related Features and Bindings** — roadmap_wave_4a, roadmap_git_diff_viewer, keybindings_git_diff, keybindings_panels_views [INFERRED 0.80]

## Communities

### Community 0 - "App State & File Ops"
Cohesion: 0.02
Nodes (164): alt_menu_shortcuts_are_available(), CollisionPolicy, Command, editor_mode_prefers_text_entry(), editor_shift_number_keys_remain_text_input(), editor_shortcuts_remain_available(), FileOperation, from_palette_key_event_handles_esc() (+156 more)

### Community 1 - "Editor Buffer & Cursor"
Cohesion: 0.02
Nodes (112): add_bookmark_persists_to_config(), apply_update_is_noop_when_no_update(), apply_update_opens_prompt_when_update_available(), AppState, batch_archive_extract_success_clears_marks_after_completed_result(), batch_full_failure_keeps_marks_and_reports_failed_status(), batch_full_success_clears_marks_and_sets_completed_status(), batch_move_success_clears_marks_after_completed_result() (+104 more)

### Community 2 - "Filesystem Backend"
Cohesion: 0.04
Nodes (81): F, LocalBackend, SftpBackend, suggest_non_conflicting_path(), PlatformPty, PtySession, which_shell(), focus_next_pane_cycles_left_to_right() (+73 more)

### Community 3 - "Actions & Keybindings"
Cohesion: 0.05
Nodes (38): cursor_moves_between_lines(), Edit, EditorBuffer, EditorError, EditorRenderState, find_matches_empty_query_returns_nothing(), find_matches_is_case_insensitive(), find_matches_returns_all_occurrences() (+30 more)

### Community 4 - "UI Menus & Overlays"
Cohesion: 0.04
Nodes (78): archive_worker_lists_zip_and_tar(), ArchiveListRequest, BackendRef, base64_encode(), build_hex_row(), connect_sftp(), describe_operation(), DirSizeRequest (+70 more)

### Community 5 - "Archive Worker"
Cohesion: 0.04
Nodes (37): annotated_config_contains_comments(), annotated_config_contains_section_headers(), annotated_config_escapes_special_chars(), annotated_config_is_valid_toml(), annotated_config_theme_preset_round_trips(), AppConfig, assert_palette_ladder(), compiles_ctrl_key_binding() (+29 more)

### Community 6 - "Config & Theme"
Cohesion: 0.05
Nodes (27): clamps_selection_at_zero(), clear_marks_removes_all(), cycle_sort_mode_wraps_around(), dir_first(), empty_pane(), filter_active_hides_non_matching_entries(), filter_empty_query_shows_all_entries(), filter_is_case_insensitive() (+19 more)

### Community 7 - "App Entry & Release Scripts"
Cohesion: 0.06
Nodes (36): main(), ok(), App, relaunch_self(), route_menu_bar_click(), route_mouse_event(), route_mouse_left_click_on_dialog_closes_it(), route_mouse_left_click_on_file_menu_opens_file_menu() (+28 more)

### Community 8 - "Pane Navigation"
Cohesion: 0.05
Nodes (63): ADR-0001 Core Architecture Decision, AGENTS.md Project Conventions, Changelog, Global Command Palette (Ctrl+P), Confirmation Modals for Destructive Actions, Dual-Pane Layout, EditorState (Editor Sub-State), ZetaError Context Propagation (+55 more)

### Community 9 - "Docs & Changelog"
Cohesion: 0.07
Nodes (21): close_all_removes_modal(), enter_flyout_not_on_trigger_switches_tab(), enter_flyout_on_trigger_activates_flyout_item(), exit_flyout_when_closed_switches_prev_tab(), exit_flyout_when_open_collapses_flyout(), flyout_trigger(), menu_activate_emits_dispatch_action(), menu_activate_on_flyout_item_dispatches_action() (+13 more)

### Community 10 - "Overlay & Modal State"
Cohesion: 0.06
Nodes (25): archive_listing_is_detected(), ArchiveEntry, ArchiveFormat, ArchiveListing, from_plain_builds_correct_total(), hex_dump_is_detected(), HexDumpData, HexRow (+17 more)

### Community 11 - "Preview System"
Cohesion: 0.07
Nodes (43): classify(), current_branch(), detect_repo(), DiffLine, DiffLineKind, fetch_diff_files(), fetch_file_diff(), fetch_status() (+35 more)

### Community 12 - "Git Integration"
Cohesion: 0.05
Nodes (28): icon_for_entry(), icon_for_kind(), nerdfont_icon(), unicode_icon(), icon_slot_ascii_returns_icon_only(), icon_slot_unicode_appends_two_spaces(), days_to_ymd(), display_width() (+20 more)

### Community 13 - "Icon System"
Cohesion: 0.08
Nodes (52): blank_line_produces_empty_line(), blockquote_uses_bar_prefix(), bold_italic_combined_applies_both_modifiers(), bullet_list_uses_bullet_char(), default_palette(), fence_lang(), fenced_block_shows_language_tag(), fenced_code_block_collects_inner_lines() (+44 more)

### Community 14 - "Markdown Renderer"
Cohesion: 0.05
Nodes (49): config.toml Keymap Configuration, Key Bindings Documentation, Editor Key Bindings, File Operations Key Bindings, File Pane Navigation Key Bindings, Git Diff Viewer Key Bindings, Global Key Bindings, Panels and Views Key Bindings (+41 more)

### Community 15 - "Tech Stack & Conventions"
Cohesion: 0.06
Nodes (44): anyhow, Cargo, crossbeam-channel, crossterm, flume, action module, app module, config module (+36 more)

### Community 16 - "SSH & Remote FS"
Cohesion: 0.06
Nodes (43): AppEvent::Mouse Variant, ArchiveWorker (6th background worker), Bookmarks in AppConfig (Vec<PathBuf>), compute_diff() — pure directory comparison, DiffStatus Enum (LeftOnly/RightOnly/Same/Different), EditorBuffer, FinderWorker (5th background worker), FocusLayer Enum (+35 more)

### Community 17 - "Update & State Types"
Cohesion: 0.06
Nodes (18): FocusLayer, MenuItem, MessageKind, ModalKind, PaneFocus, PaneLayout, status_message_error_constructor(), status_message_warning_constructor() (+10 more)

### Community 18 - "Hooks & Config Gen"
Cohesion: 0.07
Nodes (35): Annotated Config Generation, apply_view() Monolith, Command::RunHook Variant, config.toml, Contextual Hints Bar, Dual-Pane Browser, Embedded Text Editor, First-Run Wizard (+27 more)

### Community 19 - "Release Pipeline"
Cohesion: 0.11
Nodes (15): bail(), _c(), Candidate, cargo(), compute_candidates(), git(), header(), main() (+7 more)

### Community 20 - "Community 20"
Cohesion: 0.12
Nodes (26): copy_path(), copy_path_recursive(), copy_path_rejects_existing_destination(), copy_path_with_progress(), copy_path_with_progress_reports_completed_entries(), count_path_entries(), count_path_entries_counts_directories_and_files(), create_directory() (+18 more)

### Community 21 - "Community 21"
Cohesion: 0.15
Nodes (3): Action, KeyBinding, route_key_event()

### Community 22 - "Community 22"
Cohesion: 0.09
Nodes (8): CollisionState, DestructiveAction, DestructiveConfirmState, DialogState, prompt_base_path(), PromptKind, PromptState, resolve_prompt_target()

### Community 23 - "Community 23"
Cohesion: 0.18
Nodes (14): close_editor_when_dirty_keeps_buffer(), close_editor_when_not_dirty_removes_buffer(), cut_removes_selected_text(), discard_closes_buffer(), editor_state_starts_closed(), EditorState, open_markdown_file_enables_live_preview(), redo_reapplies_undone_insert() (+6 more)

### Community 24 - "Community 24"
Cohesion: 0.12
Nodes (20): Zeta Development Roadmap, App Screenshot — Workspaces & Editor, Architecture Remediation & Feature Foundation Design, AppState Decomposition, FileOperationIdentity, IconMode, WorkspaceState, Custom Icon Font Design (+12 more)

### Community 25 - "Community 25"
Cohesion: 0.19
Nodes (15): all_entries(), category_order(), filter_case_insensitive(), filter_empty_query_returns_all(), filter_entries(), filter_subsequence_matches_label(), filter_subsequence_no_match_returns_empty(), is_subsequence() (+7 more)

### Community 26 - "Community 26"
Cohesion: 0.19
Nodes (11): compute_diff(), compute_diff_different_size(), compute_diff_directories_match_by_name(), compute_diff_left_only(), compute_diff_right_only(), compute_diff_same_entry(), compute_diff_symmetric_count(), DiffStatus (+3 more)

### Community 27 - "Community 27"
Cohesion: 0.27
Nodes (15): AppConfig.hooks Field, RunHook Command Variant, Hook Configuration, Hook Runtime Context, Hook Event Enum, Wizard State Machine, Wizard Step Enum, App Exit Hook Firing (+7 more)

### Community 28 - "Community 28"
Cohesion: 0.15
Nodes (5): KeymapField, SettingsEntry, SettingsField, SettingsState, SettingsTab

### Community 29 - "Community 29"
Cohesion: 0.27
Nodes (12): format_file_size(), preview_gutter_label(), preview_gutter_label_uses_four_columns_for_line_numbers(), preview_gutter_label_uses_four_columns_for_wrapped_rows(), render_archive_preview(), render_hex_dump_preview(), render_image_preview(), render_preview_panel() (+4 more)

### Community 30 - "Community 30"
Cohesion: 0.31
Nodes (8): compute_scan_diff(), detects_added_entry(), detects_modified_entry_by_mtime(), detects_modified_entry_by_size(), detects_removed_entry(), empty_diff_when_no_changes(), entry(), ScanDiff

### Community 31 - "Community 31"
Cohesion: 0.22
Nodes (5): HostKeyFingerprints, SshAuthMethod, SshConnectionState, SshDialogField, SshErrorKind

### Community 32 - "Community 32"
Cohesion: 0.36
Nodes (7): build_row_spans(), char_highlight_bg(), CodeViewRenderArgs, render_code_view(), SearchHighlight, SelectionHighlight, styled_span()

### Community 33 - "Community 33"
Cohesion: 0.67
Nodes (2): CopyProgress, FsBackend

### Community 34 - "Community 34"
Cohesion: 0.67
Nodes (3): StatusZones (zoned status bar), ThemePalette (Catppuccin Mocha + accent tokens), UI/UX Revamp (Catppuccin + NerdFont)

### Community 35 - "Community 35"
Cohesion: 1.0
Nodes (3): UpdateChecker (GitHub API + self-update), Update Checks & Self-Update, ureq crate v2.9 (HTTP client)

### Community 36 - "Community 36"
Cohesion: 0.67
Nodes (3): FileOperationIdentity / FileOperationKind, File Operation Identity Hardening, Operation Safety Hardening

### Community 37 - "Community 37"
Cohesion: 1.0
Nodes (1): Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach

### Community 38 - "Community 38"
Cohesion: 1.0
Nodes (1): Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s

### Community 39 - "Community 39"
Cohesion: 1.0
Nodes (1): Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach

### Community 40 - "Community 40"
Cohesion: 1.0
Nodes (1): Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca

### Community 41 - "Community 41"
Cohesion: 1.0
Nodes (1): AppEvent

### Community 44 - "Community 44"
Cohesion: 1.0
Nodes (2): ModalState::Menu flyout extension, Flyout Submenu (View→Themes)

### Community 45 - "Community 45"
Cohesion: 1.0
Nodes (2): ViewBuffer, Preview Enhancements Design

### Community 58 - "Community 58"
Cohesion: 1.0
Nodes (1): Performance Baseline

### Community 59 - "Community 59"
Cohesion: 1.0
Nodes (1): Release Flow Documentation

### Community 60 - "Community 60"
Cohesion: 1.0
Nodes (1): Terminal Behavior Guide

### Community 62 - "Community 62"
Cohesion: 1.0
Nodes (1): Job Result Sender

### Community 63 - "Community 63"
Cohesion: 1.0
Nodes (1): Wizard State Module

### Community 64 - "Community 64"
Cohesion: 1.0
Nodes (1): Wave 2A: Input Routing (FocusLayer)

### Community 65 - "Community 65"
Cohesion: 1.0
Nodes (1): Wave 2B: Mouse Support

### Community 66 - "Community 66"
Cohesion: 1.0
Nodes (1): Wave 4D: Quick Filter + Fuzzy File Find

### Community 67 - "Community 67"
Cohesion: 1.0
Nodes (1): Wave 5A: Find & Replace + Directory Watcher

### Community 68 - "Community 68"
Cohesion: 1.0
Nodes (1): Wave 5B: Bookmarks + Trash

### Community 69 - "Community 69"
Cohesion: 1.0
Nodes (1): Wave 6A: Archive Browsing

## Knowledge Gaps
- **205 isolated node(s):** `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`, `EditorRenderState` (+200 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 33`** (3 nodes): `CopyProgress`, `FsBackend`, `backend.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 37`** (2 nodes): `patch_cache_populate.py`, `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 38`** (2 nodes): `patch_cache_refresh.py`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 39`** (2 nodes): `patch_cache_tests.py`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 40`** (2 nodes): `patch_scan_cache_pane.py`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 41`** (2 nodes): `AppEvent`, `event.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 44`** (2 nodes): `ModalState::Menu flyout extension`, `Flyout Submenu (View→Themes)`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 45`** (2 nodes): `ViewBuffer`, `Preview Enhancements Design`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 58`** (1 nodes): `Performance Baseline`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 59`** (1 nodes): `Release Flow Documentation`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 60`** (1 nodes): `Terminal Behavior Guide`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 62`** (1 nodes): `Job Result Sender`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 63`** (1 nodes): `Wizard State Module`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 64`** (1 nodes): `Wave 2A: Input Routing (FocusLayer)`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 65`** (1 nodes): `Wave 2B: Mouse Support`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 66`** (1 nodes): `Wave 4D: Quick Filter + Fuzzy File Find`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 67`** (1 nodes): `Wave 5A: Find & Replace + Directory Watcher`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 68`** (1 nodes): `Wave 5B: Bookmarks + Trash`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 69`** (1 nodes): `Wave 6A: Archive Browsing`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ok()` connect `App Entry & Release Scripts` to `App State & File Ops`, `Editor Buffer & Cursor`, `Filesystem Backend`, `Actions & Keybindings`, `Config & Theme`, `Overlay & Modal State`, `Preview System`, `Release Pipeline`, `Community 20`, `Community 23`?**
  _High betweenness centrality (0.104) - this node is a cross-community bridge._
- **Why does `AppState` connect `Editor Buffer & Cursor` to `App State & File Ops`, `Update & State Types`, `Preview System`?**
  _High betweenness centrality (0.083) - this node is a cross-community bridge._
- **Why does `test_state()` connect `Editor Buffer & Cursor` to `App State & File Ops`, `Archive Worker`?**
  _High betweenness centrality (0.052) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `test_state()` (e.g. with `.resolve()` and `.default()`) actually correct?**
  _`test_state()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **Are the 63 inferred relationships involving `ok()` (e.g. with `.open()` and `.save()`) actually correct?**
  _`ok()` has 63 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach` to the rest of the system?**
  _205 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `App State & File Ops` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
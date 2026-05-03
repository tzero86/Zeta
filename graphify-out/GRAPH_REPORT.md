# Graph Report - .  (2026-05-03)

## Corpus Check
- 178 files · ~150,000 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2122 nodes · 4274 edges · 60 communities detected
- Extraction: 78% EXTRACTED · 22% INFERRED · 0% AMBIGUOUS · INFERRED: 958 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_TUI Test Suite|TUI Test Suite]]
- [[_COMMUNITY_State Management Core|State Management Core]]
- [[_COMMUNITY_Editor Engine|Editor Engine]]
- [[_COMMUNITY_Pane Navigation|Pane Navigation]]
- [[_COMMUNITY_Filesystem Operations|Filesystem Operations]]
- [[_COMMUNITY_UI Rendering|UI Rendering]]
- [[_COMMUNITY_Configuration & Keymap|Configuration & Keymap]]
- [[_COMMUNITY_Job System|Job System]]
- [[_COMMUNITY_Preview & Git Diff|Preview & Git Diff]]
- [[_COMMUNITY_SSH Remote|SSH Remote]]
- [[_COMMUNITY_Action Dispatch|Action Dispatch]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
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
- [[_COMMUNITY_Community 42|Community 42]]
- [[_COMMUNITY_Community 43|Community 43]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 65|Community 65]]
- [[_COMMUNITY_Community 66|Community 66]]
- [[_COMMUNITY_Community 67|Community 67]]
- [[_COMMUNITY_Community 69|Community 69]]
- [[_COMMUNITY_Community 70|Community 70]]
- [[_COMMUNITY_Community 71|Community 71]]
- [[_COMMUNITY_Community 72|Community 72]]
- [[_COMMUNITY_Community 73|Community 73]]
- [[_COMMUNITY_Community 74|Community 74]]
- [[_COMMUNITY_Community 75|Community 75]]
- [[_COMMUNITY_Community 76|Community 76]]
- [[_COMMUNITY_Community 77|Community 77]]
- [[_COMMUNITY_Community 78|Community 78]]

## God Nodes (most connected - your core abstractions)
1. `test_state()` - 111 edges
2. `AppState` - 105 edges
3. `ok()` - 67 edges
4. `EditorBuffer` - 49 edges
5. `PaneState` - 44 edges
6. `OverlayState` - 40 edges
7. `render()` - 36 edges
8. `parse_markdown_lines()` - 35 edges
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
- **Zeta TUI Integration Test Suite** — smoke_test, navigation_test, cheatsheet_test, editor_test, filter_test, files_test, overlays_test, preview_test, workspaces_test, tui_test_config [EXTRACTED 1.00]
- **GoTo Path Navigation Helper Pattern** — editor_test, files_test, concept_goto_path_prompt [EXTRACTED 1.00]
- **Context-Aware Cheatsheet Overlay** — concept_cheatsheet_overlay, concept_editor_mode, concept_file_manager_ui [INFERRED 0.85]

## Communities

### Community 0 - "TUI Test Suite"
Cohesion: 0.02
Nodes (180): alt_menu_shortcuts_are_available(), CollisionPolicy, Command, editor_mode_prefers_text_entry(), editor_shift_number_keys_remain_text_input(), editor_shortcuts_remain_available(), FileOperation, from_palette_key_event_handles_esc() (+172 more)

### Community 1 - "State Management Core"
Cohesion: 0.02
Nodes (120): add_bookmark_persists_to_config(), apply_update_is_noop_when_no_update(), apply_update_opens_prompt_when_update_available(), AppState, batch_archive_extract_success_clears_marks_after_completed_result(), batch_full_failure_keeps_marks_and_reports_failed_status(), batch_full_success_clears_marks_and_sets_completed_status(), batch_move_success_clears_marks_after_completed_result() (+112 more)

### Community 2 - "Editor Engine"
Cohesion: 0.04
Nodes (95): F, create_file(), suggest_non_conflicting_path(), focus_next_pane_cycles_left_to_right(), focus_next_pane_cycles_right_to_left(), inactive_pane_returns_opposite_of_focus(), make_state(), PaneSetState (+87 more)

### Community 3 - "Pane Navigation"
Cohesion: 0.02
Nodes (124): AppState, ArchiveListing, ArchiveWorker (6th background worker), Bookmarks in AppConfig (Vec<PathBuf>), compute_diff() — pure directory comparison, DiffStatus Enum (LeftOnly/RightOnly/Same/Different), EditorBuffer, FileOperationKind (+116 more)

### Community 4 - "Filesystem Operations"
Cohesion: 0.04
Nodes (29): clamps_selection_at_zero(), clear_marks_removes_all(), cycle_sort_mode_wraps_around(), dir_first(), empty_pane(), filter_active_hides_non_matching_entries(), filter_empty_query_shows_all_entries(), filter_is_case_insensitive() (+21 more)

### Community 5 - "UI Rendering"
Cohesion: 0.05
Nodes (38): cursor_moves_between_lines(), Edit, EditorBuffer, EditorError, EditorRenderState, find_matches_empty_query_returns_nothing(), find_matches_is_case_insensitive(), find_matches_returns_all_occurrences() (+30 more)

### Community 6 - "Configuration & Keymap"
Cohesion: 0.05
Nodes (23): main(), LocalBackend, SftpBackend, ok(), App, relaunch_self(), run_update_and_restart(), TerminalSession (+15 more)

### Community 7 - "Job System"
Cohesion: 0.04
Nodes (75): ArchiveListRequest, BackendRef, base64_encode(), build_hex_row(), connect_sftp(), describe_operation(), DirSizeRequest, EditorLoadRequest (+67 more)

### Community 8 - "Preview & Git Diff"
Cohesion: 0.04
Nodes (37): annotated_config_contains_comments(), annotated_config_contains_section_headers(), annotated_config_escapes_special_chars(), annotated_config_is_valid_toml(), annotated_config_theme_preset_round_trips(), AppConfig, assert_palette_ladder(), compiles_ctrl_key_binding() (+29 more)

### Community 9 - "SSH Remote"
Cohesion: 0.03
Nodes (76): ADR-0001 Core Architecture, crossbeam-channel crate, crossterm crate, Event-Action-Reducer Flow, Modular Monolith Architecture, action module, app module, config module (+68 more)

### Community 10 - "Action Dispatch"
Cohesion: 0.06
Nodes (27): menu_items_for(), menu_tabs(), MenuContext, MenuTab, navigate_menu_starts_with_workspace_switch_items(), close_all_removes_modal(), ContextMenuItem, enter_flyout_not_on_trigger_switches_tab() (+19 more)

### Community 11 - "Community 11"
Cohesion: 0.05
Nodes (63): ADR-0001 Core Architecture Decision, AGENTS.md Project Conventions, Changelog, Global Command Palette (Ctrl+P), Confirmation Modals for Destructive Actions, Dual-Pane Layout, EditorState (Editor Sub-State), ZetaError Context Propagation (+55 more)

### Community 12 - "Community 12"
Cohesion: 0.06
Nodes (25): archive_listing_is_detected(), ArchiveEntry, ArchiveFormat, ArchiveListing, from_plain_builds_correct_total(), hex_dump_is_detected(), HexDumpData, HexRow (+17 more)

### Community 13 - "Community 13"
Cohesion: 0.07
Nodes (43): classify(), current_branch(), detect_repo(), DiffLine, DiffLineKind, fetch_diff_files(), fetch_file_diff(), fetch_status() (+35 more)

### Community 14 - "Community 14"
Cohesion: 0.08
Nodes (52): blank_line_produces_empty_line(), blockquote_uses_bar_prefix(), bold_italic_combined_applies_both_modifiers(), bullet_list_uses_bullet_char(), default_palette(), fence_lang(), fenced_block_shows_language_tag(), fenced_code_block_collects_inner_lines() (+44 more)

### Community 15 - "Community 15"
Cohesion: 0.05
Nodes (49): config.toml Keymap Configuration, Key Bindings Documentation, Editor Key Bindings, File Operations Key Bindings, File Pane Navigation Key Bindings, Git Diff Viewer Key Bindings, Global Key Bindings, Panels and Views Key Bindings (+41 more)

### Community 16 - "Community 16"
Cohesion: 0.06
Nodes (44): anyhow, Cargo, crossbeam-channel, crossterm, flume, action module, app module, config module (+36 more)

### Community 17 - "Community 17"
Cohesion: 0.06
Nodes (18): FocusLayer, MenuItem, MessageKind, ModalKind, PaneFocus, PaneLayout, status_message_error_constructor(), status_message_warning_constructor() (+10 more)

### Community 18 - "Community 18"
Cohesion: 0.09
Nodes (35): AppEvent::Mouse Variant, Architecture Remediation and Feature Foundation Design, Flyout Submenu State (ModalState::Menu flyout field), Flyout Submenu (View→Themes), Flyout Submenu Design Spec, FocusLayer Enum, Git Diff Viewer Feature, Git Diff Viewer Design Spec (+27 more)

### Community 19 - "Community 19"
Cohesion: 0.09
Nodes (26): icon_for_entry(), icon_for_kind(), nerdfont_icon(), unicode_icon(), DimOverlay, days_to_ymd(), display_width(), format_entry_meta() (+18 more)

### Community 20 - "Community 20"
Cohesion: 0.07
Nodes (35): Annotated Config Generation, apply_view() Monolith, Command::RunHook Variant, config.toml, Contextual Hints Bar, Dual-Pane Browser, Embedded Text Editor, First-Run Wizard (+27 more)

### Community 21 - "Community 21"
Cohesion: 0.11
Nodes (15): bail(), _c(), Candidate, cargo(), compute_candidates(), git(), header(), main() (+7 more)

### Community 22 - "Community 22"
Cohesion: 0.08
Nodes (32): Cheatsheet Test Suite, Bookmarks Overlay (Alt+N then k), Cheatsheet Overlay (? key / Quick Reference), Command Palette Overlay (Shift+P), Context Menu (Shift+F10), Embedded Editor Mode (F4), File Finder Overlay (Ctrl+P), Dual-Pane File Manager UI (+24 more)

### Community 23 - "Community 23"
Cohesion: 0.12
Nodes (25): copy_path(), copy_path_recursive(), copy_path_rejects_existing_destination(), copy_path_with_progress(), copy_path_with_progress_reports_completed_entries(), count_path_entries(), count_path_entries_counts_directories_and_files(), create_directory() (+17 more)

### Community 24 - "Community 24"
Cohesion: 0.15
Nodes (3): Action, KeyBinding, route_key_event()

### Community 25 - "Community 25"
Cohesion: 0.09
Nodes (8): CollisionState, DestructiveAction, DestructiveConfirmState, DialogState, prompt_base_path(), PromptKind, PromptState, resolve_prompt_target()

### Community 26 - "Community 26"
Cohesion: 0.12
Nodes (20): Zeta Development Roadmap, App Screenshot — Workspaces & Editor, Architecture Remediation & Feature Foundation Design, AppState Decomposition, FileOperationIdentity, IconMode, WorkspaceState, Custom Icon Font Design (+12 more)

### Community 27 - "Community 27"
Cohesion: 0.19
Nodes (15): all_entries(), category_order(), filter_case_insensitive(), filter_empty_query_returns_all(), filter_entries(), filter_subsequence_matches_label(), filter_subsequence_no_match_returns_empty(), is_subsequence() (+7 more)

### Community 28 - "Community 28"
Cohesion: 0.19
Nodes (11): compute_diff(), compute_diff_different_size(), compute_diff_directories_match_by_name(), compute_diff_left_only(), compute_diff_right_only(), compute_diff_same_entry(), compute_diff_symmetric_count(), DiffStatus (+3 more)

### Community 29 - "Community 29"
Cohesion: 0.27
Nodes (15): AppConfig.hooks Field, RunHook Command Variant, Hook Configuration, Hook Runtime Context, Hook Event Enum, Wizard State Machine, Wizard Step Enum, App Exit Hook Firing (+7 more)

### Community 30 - "Community 30"
Cohesion: 0.15
Nodes (5): KeymapField, SettingsEntry, SettingsField, SettingsState, SettingsTab

### Community 31 - "Community 31"
Cohesion: 0.31
Nodes (8): compute_scan_diff(), detects_added_entry(), detects_modified_entry_by_mtime(), detects_modified_entry_by_size(), detects_removed_entry(), empty_diff_when_no_changes(), entry(), ScanDiff

### Community 32 - "Community 32"
Cohesion: 0.22
Nodes (5): HostKeyFingerprints, SshAuthMethod, SshConnectionState, SshDialogField, SshErrorKind

### Community 33 - "Community 33"
Cohesion: 0.38
Nodes (7): Custom Icon Font Design Spec, IconMode Enum, Neo-Commander UI/UX Design Spec, StatusZones (zoned status bar), Theme Palette (Catppuccin Mocha), UI/UX Revamp (Catppuccin + NerdFont), UI/UX Revamp Design Spec

### Community 34 - "Community 34"
Cohesion: 0.4
Nodes (5): CatppuccinMocha Theme Preset, modal_halo Colour Token, UI/UX Revamp Plan, StatusZones (Zoned Status Bar), ThemePalette Expansion (Accent Tokens)

### Community 35 - "Community 35"
Cohesion: 0.5
Nodes (4): AppEvent::Mouse Variant, EnableMouseCapture / DisableMouseCapture, Wave 2B: Full Mouse Support Plan, route_mouse_event Function

### Community 36 - "Community 36"
Cohesion: 0.83
Nodes (4): File Operation Identity Hardening Design, FileOperationIdentity, Operation Safety Hardening Design, PendingBatchOperation

### Community 37 - "Community 37"
Cohesion: 0.5
Nodes (4): File Marks (Space key, › * indicator), Pane Filter (/ key, inline filter bar), Sort Cycling (s key), Filter and Sort Test Suite

### Community 38 - "Community 38"
Cohesion: 0.67
Nodes (2): CopyProgress, FsBackend

### Community 39 - "Community 39"
Cohesion: 0.67
Nodes (3): FileOperationIdentity / FileOperationKind, File Operation Identity Hardening, Operation Safety Hardening

### Community 40 - "Community 40"
Cohesion: 0.67
Nodes (3): MenuEnterFlyout / MenuExitFlyout Actions, ModalState::Menu flyout Extension, Flyout Submenu Plan (View → Themes)

### Community 41 - "Community 41"
Cohesion: 1.0
Nodes (1): Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach

### Community 42 - "Community 42"
Cohesion: 1.0
Nodes (1): Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s

### Community 43 - "Community 43"
Cohesion: 1.0
Nodes (1): Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach

### Community 44 - "Community 44"
Cohesion: 1.0
Nodes (1): Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca

### Community 45 - "Community 45"
Cohesion: 1.0
Nodes (1): AppEvent

### Community 48 - "Community 48"
Cohesion: 1.0
Nodes (2): ViewBuffer, Preview Enhancements Design

### Community 65 - "Community 65"
Cohesion: 1.0
Nodes (1): Performance Baseline

### Community 66 - "Community 66"
Cohesion: 1.0
Nodes (1): Release Flow Documentation

### Community 67 - "Community 67"
Cohesion: 1.0
Nodes (1): Terminal Behavior Guide

### Community 69 - "Community 69"
Cohesion: 1.0
Nodes (1): Job Result Sender

### Community 70 - "Community 70"
Cohesion: 1.0
Nodes (1): Wizard State Module

### Community 71 - "Community 71"
Cohesion: 1.0
Nodes (1): Wave 2A: Input Routing (FocusLayer)

### Community 72 - "Community 72"
Cohesion: 1.0
Nodes (1): Wave 2B: Mouse Support

### Community 73 - "Community 73"
Cohesion: 1.0
Nodes (1): Wave 4D: Quick Filter + Fuzzy File Find

### Community 74 - "Community 74"
Cohesion: 1.0
Nodes (1): Wave 5A: Find & Replace + Directory Watcher

### Community 75 - "Community 75"
Cohesion: 1.0
Nodes (1): Wave 5B: Bookmarks + Trash

### Community 76 - "Community 76"
Cohesion: 1.0
Nodes (1): Wave 6A: Archive Browsing

### Community 77 - "Community 77"
Cohesion: 1.0
Nodes (1): Live Clock Status Bar

### Community 78 - "Community 78"
Cohesion: 1.0
Nodes (1): ThemePalette v2

## Knowledge Gaps
- **281 isolated node(s):** `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`, `EditorRenderState` (+276 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 38`** (3 nodes): `CopyProgress`, `FsBackend`, `backend.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 41`** (2 nodes): `patch_cache_populate.py`, `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 42`** (2 nodes): `patch_cache_refresh.py`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 43`** (2 nodes): `patch_cache_tests.py`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 44`** (2 nodes): `patch_scan_cache_pane.py`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 45`** (2 nodes): `AppEvent`, `event.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 48`** (2 nodes): `ViewBuffer`, `Preview Enhancements Design`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 65`** (1 nodes): `Performance Baseline`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 66`** (1 nodes): `Release Flow Documentation`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 67`** (1 nodes): `Terminal Behavior Guide`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 69`** (1 nodes): `Job Result Sender`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 70`** (1 nodes): `Wizard State Module`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 71`** (1 nodes): `Wave 2A: Input Routing (FocusLayer)`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 72`** (1 nodes): `Wave 2B: Mouse Support`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 73`** (1 nodes): `Wave 4D: Quick Filter + Fuzzy File Find`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 74`** (1 nodes): `Wave 5A: Find & Replace + Directory Watcher`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 75`** (1 nodes): `Wave 5B: Bookmarks + Trash`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 76`** (1 nodes): `Wave 6A: Archive Browsing`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 77`** (1 nodes): `Live Clock Status Bar`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 78`** (1 nodes): `ThemePalette v2`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ok()` connect `Configuration & Keymap` to `TUI Test Suite`, `State Management Core`, `Editor Engine`, `UI Rendering`, `Action Dispatch`, `Community 12`, `Community 13`, `Community 21`, `Community 23`?**
  _High betweenness centrality (0.101) - this node is a cross-community bridge._
- **Why does `Wave 2B: Full Mouse Support` connect `Community 18` to `Pane Navigation`?**
  _High betweenness centrality (0.077) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `test_state()` (e.g. with `.resolve()` and `.default()`) actually correct?**
  _`test_state()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **Are the 64 inferred relationships involving `ok()` (e.g. with `.open()` and `.save()`) actually correct?**
  _`ok()` has 64 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach` to the rest of the system?**
  _281 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `TUI Test Suite` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
- **Should `State Management Core` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
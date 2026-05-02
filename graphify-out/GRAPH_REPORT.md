# Graph Report - .  (2026-05-02)

## Corpus Check
- 143 files · ~219,961 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1607 nodes · 3440 edges · 39 communities detected
- Extraction: 77% EXTRACTED · 23% INFERRED · 0% AMBIGUOUS · INFERRED: 787 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Core State Modules|Core State Modules]]
- [[_COMMUNITY_Filesystem Backend Operations|Filesystem Backend Operations]]
- [[_COMMUNITY_Application Orchestration|Application Orchestration]]
- [[_COMMUNITY_State Reducers & Jobs|State Reducers & Jobs]]
- [[_COMMUNITY_File Copy Operations|File Copy Operations]]
- [[_COMMUNITY_Pane Navigation & Selection|Pane Navigation & Selection]]
- [[_COMMUNITY_Editor Buffer & Cursor|Editor Buffer & Cursor]]
- [[_COMMUNITY_App State & Key Events|App State & Key Events]]
- [[_COMMUNITY_Architecture Docs & Features|Architecture Docs & Features]]
- [[_COMMUNITY_Menu & Overlay UI|Menu & Overlay UI]]
- [[_COMMUNITY_Action & Input Routing|Action & Input Routing]]
- [[_COMMUNITY_Preview & Archive|Preview & Archive]]
- [[_COMMUNITY_Git Integration|Git Integration]]
- [[_COMMUNITY_Markdown Rendering|Markdown Rendering]]
- [[_COMMUNITY_Events & Background Workers|Events & Background Workers]]
- [[_COMMUNITY_Icon System|Icon System]]
- [[_COMMUNITY_Fuzzy Find & Candidates|Fuzzy Find & Candidates]]
- [[_COMMUNITY_Entry Filtering|Entry Filtering]]
- [[_COMMUNITY_Dialogs & Prompts|Dialogs & Prompts]]
- [[_COMMUNITY_Design Docs & Roadmap|Design Docs & Roadmap]]
- [[_COMMUNITY_Directory Diff|Directory Diff]]
- [[_COMMUNITY_Pane Set & Focus|Pane Set & Focus]]
- [[_COMMUNITY_Settings & Keymap|Settings & Keymap]]
- [[_COMMUNITY_Focus Layer Types|Focus Layer Types]]
- [[_COMMUNITY_Version & Update|Version & Update]]
- [[_COMMUNITY_Scan Cache Diffing|Scan Cache Diffing]]
- [[_COMMUNITY_SSH State|SSH State]]
- [[_COMMUNITY_FS Backend Interface|FS Backend Interface]]
- [[_COMMUNITY_Self-Update System|Self-Update System]]
- [[_COMMUNITY_File Op Safety|File Op Safety]]
- [[_COMMUNITY_Theme & UI Revamp|Theme & UI Revamp]]
- [[_COMMUNITY_Cache Populate Patch|Cache Populate Patch]]
- [[_COMMUNITY_Cache Refresh Patch|Cache Refresh Patch]]
- [[_COMMUNITY_Cache Test Patches|Cache Test Patches]]
- [[_COMMUNITY_Scan Cache Pane Patch|Scan Cache Pane Patch]]
- [[_COMMUNITY_App Event System|App Event System]]
- [[_COMMUNITY_Terminal Integration|Terminal Integration]]
- [[_COMMUNITY_Flyout Submenu|Flyout Submenu]]
- [[_COMMUNITY_Preview Buffer|Preview Buffer]]

## God Nodes (most connected - your core abstractions)
1. `test_state()` - 95 edges
2. `AppState` - 88 edges
3. `ok()` - 71 edges
4. `EditorBuffer` - 49 edges
5. `PaneState` - 44 edges
6. `OverlayState` - 37 edges
7. `parse_markdown_lines()` - 35 edges
8. `route_key_event()` - 32 edges
9. `render()` - 30 edges
10. `spawn_workers()` - 26 edges

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
- **Wave 1 Parallel Refactoring Plans (1A+1B+1C)** — plan_wave1a, plan_wave1b, plan_wave1c [EXTRACTED 0.95]
- **AppState Decomposition into Four Sub-States** — concept_panesetstate, concept_editorstate, concept_previewstate, concept_overlaystate [EXTRACTED 1.00]
- **SSH Security Feature Cluster** — concept_host_key_fingerprints, concept_ssh_agent_detection, feature_ssh_sftp [INFERRED 0.85]
- **Background Worker System (all workers share JobResult channel)** — git_worker, finder_worker, archive_worker, watcher_worker, terminal_worker [INFERRED 0.88]
- **Editor Feature Stack (Rope + Find/Replace + Fullscreen)** — wave3a_rope_backend, wave4c_editor_fullscreen, wave5a_find_replace_watcher [INFERRED 0.82]
- **File Operation Hardening (identity + safety + workspace routing)** — fileop_identity_hardening, operation_safety_hardening, workspaces_plan [INFERRED 0.75]
- **File Operation Safety System** — operationSafetyHardening_OperationSafetyHardeningDesign, fileOpIdentityHardening_FileOpIdentityHardeningDesign, concept_FileOperationIdentity [EXTRACTED 0.95]
- **Workspace Isolation Architecture** — workspacesDesign_WorkspacesDesign, concept_WorkspaceState, gitDiffViewer_GitDiffViewerDesign [EXTRACTED 0.90]
- **Visual Design Overhaul** — neoCommanderDesign_NeoCommanderUIDesign, uiUxRevamp_UIUXRevampDesign, concept_IconMode [INFERRED 0.82]

## Communities

### Community 0 - "Core State Modules"
Cohesion: 0.03
Nodes (115): entry(), BookmarksState, close_editor_when_dirty_keeps_buffer(), close_editor_when_not_dirty_removes_buffer(), cut_removes_selected_text(), discard_closes_buffer(), editor_state_starts_closed(), EditorState (+107 more)

### Community 1 - "Filesystem Backend Operations"
Cohesion: 0.03
Nodes (91): F, LocalBackend, SftpBackend, alt_menu_shortcuts_are_available(), CollisionPolicy, Command, editor_mode_prefers_text_entry(), editor_shift_number_keys_remain_text_input() (+83 more)

### Community 2 - "Application Orchestration"
Cohesion: 0.03
Nodes (50): main(), ok(), App, TerminalSession, AppConfig, assert_palette_ladder(), compiles_ctrl_key_binding(), ConfigError (+42 more)

### Community 3 - "State Reducers & Jobs"
Cohesion: 0.05
Nodes (104): add_bookmark_persists_to_config(), batch_archive_extract_success_clears_marks_after_completed_result(), batch_full_failure_keeps_marks_and_reports_failed_status(), batch_full_success_clears_marks_and_sets_completed_status(), batch_move_success_clears_marks_after_completed_result(), batch_partial_failure_keeps_failed_marks_only(), batch_prompt_submit_does_not_clear_marks_at_dispatch(), bootstrap_initial_commands_queue_both_pane_scans() (+96 more)

### Community 4 - "File Copy Operations"
Cohesion: 0.03
Nodes (100): copy_path(), copy_path_recursive(), copy_path_rejects_existing_destination(), copy_path_with_progress(), copy_path_with_progress_reports_completed_entries(), count_path_entries(), count_path_entries_counts_directories_and_files(), create_directory() (+92 more)

### Community 5 - "Pane Navigation & Selection"
Cohesion: 0.04
Nodes (29): clamps_selection_at_zero(), clear_marks_removes_all(), cycle_sort_mode_wraps_around(), dir_first(), empty_pane(), filter_active_hides_non_matching_entries(), filter_empty_query_shows_all_entries(), filter_is_case_insensitive() (+21 more)

### Community 6 - "Editor Buffer & Cursor"
Cohesion: 0.05
Nodes (38): cursor_moves_between_lines(), Edit, EditorBuffer, EditorError, EditorRenderState, find_matches_empty_query_returns_nothing(), find_matches_is_case_insensitive(), find_matches_returns_all_occurrences() (+30 more)

### Community 7 - "App State & Key Events"
Cohesion: 0.04
Nodes (2): key_event_to_string(), AppState

### Community 8 - "Architecture Docs & Features"
Cohesion: 0.05
Nodes (69): ADR-0001 Core Architecture Decision, AGENTS.md Project Conventions, Changelog, Global Command Palette (Ctrl+P), Confirmation Modals for Destructive Actions, Dual-Pane Layout, EditorState (Editor Sub-State), ZetaError Context Propagation (+61 more)

### Community 9 - "Menu & Overlay UI"
Cohesion: 0.06
Nodes (25): menu_items_for(), MenuContext, MenuTab, navigate_menu_starts_with_workspace_switch_items(), close_all_removes_modal(), enter_flyout_not_on_trigger_switches_tab(), enter_flyout_on_trigger_activates_flyout_item(), exit_flyout_when_closed_switches_prev_tab() (+17 more)

### Community 10 - "Action & Input Routing"
Cohesion: 0.07
Nodes (27): Action, bookmarks_layer_routes_enter_to_confirm_selection(), command_palette_remains_available_while_editor_is_open(), editor_layer_ctrl_f_opens_search(), editor_shortcuts_still_take_priority_over_global_fallbacks(), palette_layer_routes_esc_to_close_palette(), palette_open_state_blocks_lower_priority_input_paths(), pane_layer_ctrl_q_quits() (+19 more)

### Community 11 - "Preview & Archive"
Cohesion: 0.06
Nodes (26): load_image_preview(), archive_listing_is_detected(), ArchiveEntry, ArchiveFormat, ArchiveListing, from_plain_builds_correct_total(), hex_dump_is_detected(), HexDumpData (+18 more)

### Community 12 - "Git Integration"
Cohesion: 0.07
Nodes (43): classify(), current_branch(), detect_repo(), DiffLine, DiffLineKind, fetch_diff_files(), fetch_file_diff(), fetch_status() (+35 more)

### Community 13 - "Markdown Rendering"
Cohesion: 0.08
Nodes (51): blank_line_produces_empty_line(), blockquote_uses_bar_prefix(), bold_italic_combined_applies_both_modifiers(), bullet_list_uses_bullet_char(), default_palette(), fence_lang(), fenced_block_shows_language_tag(), fenced_code_block_collects_inner_lines() (+43 more)

### Community 14 - "Events & Background Workers"
Cohesion: 0.06
Nodes (43): AppEvent::Mouse Variant, ArchiveWorker (6th background worker), Bookmarks in AppConfig (Vec<PathBuf>), compute_diff() — pure directory comparison, DiffStatus Enum (LeftOnly/RightOnly/Same/Different), EditorBuffer, FinderWorker (5th background worker), FocusLayer Enum (+35 more)

### Community 15 - "Icon System"
Cohesion: 0.08
Nodes (27): icon_for_entry(), icon_for_kind(), nerdfont_icon(), unicode_icon(), icon_slot_ascii_returns_icon_only(), icon_slot_unicode_appends_two_spaces(), DimOverlay, days_to_ymd() (+19 more)

### Community 16 - "Fuzzy Find & Candidates"
Cohesion: 0.11
Nodes (15): bail(), _c(), Candidate, cargo(), compute_candidates(), git(), header(), main() (+7 more)

### Community 17 - "Entry Filtering"
Cohesion: 0.13
Nodes (19): all_entries(), category_order(), filter_case_insensitive(), filter_empty_query_returns_all(), filter_entries(), filter_subsequence_matches_label(), filter_subsequence_no_match_returns_empty(), is_subsequence() (+11 more)

### Community 18 - "Dialogs & Prompts"
Cohesion: 0.09
Nodes (8): CollisionState, DestructiveAction, DestructiveConfirmState, DialogState, prompt_base_path(), PromptKind, PromptState, resolve_prompt_target()

### Community 19 - "Design Docs & Roadmap"
Cohesion: 0.12
Nodes (20): Zeta Development Roadmap, App Screenshot — Workspaces & Editor, Architecture Remediation & Feature Foundation Design, AppState Decomposition, FileOperationIdentity, IconMode, WorkspaceState, Custom Icon Font Design (+12 more)

### Community 20 - "Directory Diff"
Cohesion: 0.19
Nodes (11): compute_diff(), compute_diff_different_size(), compute_diff_directories_match_by_name(), compute_diff_left_only(), compute_diff_right_only(), compute_diff_same_entry(), compute_diff_symmetric_count(), DiffStatus (+3 more)

### Community 21 - "Pane Set & Focus"
Cohesion: 0.22
Nodes (8): focus_next_pane_cycles_left_to_right(), focus_next_pane_cycles_right_to_left(), inactive_pane_returns_opposite_of_focus(), make_state(), PaneSetState, refresh_with_fresh_cache_skips_scan(), refresh_with_no_cache_queues_scan(), refresh_with_stale_mtime_queues_scan()

### Community 22 - "Settings & Keymap"
Cohesion: 0.15
Nodes (5): KeymapField, SettingsEntry, SettingsField, SettingsState, SettingsTab

### Community 23 - "Focus Layer Types"
Cohesion: 0.17
Nodes (8): FocusLayer, MenuItem, ModalKind, PaneFocus, PaneLayout, zeta_error_fs_displays_context(), zeta_error_other_displays_message(), ZetaError

### Community 24 - "Version & Update"
Cohesion: 0.24
Nodes (6): is_newer_version(), parse_version_tag(), Release, UpdateChecker, UpdateError, UpdateStatus

### Community 25 - "Scan Cache Diffing"
Cohesion: 0.36
Nodes (7): compute_scan_diff(), detects_added_entry(), detects_modified_entry_by_mtime(), detects_modified_entry_by_size(), detects_removed_entry(), empty_diff_when_no_changes(), ScanDiff

### Community 26 - "SSH State"
Cohesion: 0.22
Nodes (5): HostKeyFingerprints, SshAuthMethod, SshConnectionState, SshDialogField, SshErrorKind

### Community 27 - "FS Backend Interface"
Cohesion: 0.67
Nodes (2): CopyProgress, FsBackend

### Community 28 - "Self-Update System"
Cohesion: 1.0
Nodes (3): UpdateChecker (GitHub API + self-update), Update Checks & Self-Update, ureq crate v2.9 (HTTP client)

### Community 29 - "File Op Safety"
Cohesion: 0.67
Nodes (3): FileOperationIdentity / FileOperationKind, File Operation Identity Hardening, Operation Safety Hardening

### Community 30 - "Theme & UI Revamp"
Cohesion: 0.67
Nodes (3): StatusZones (zoned status bar), ThemePalette (Catppuccin Mocha + accent tokens), UI/UX Revamp (Catppuccin + NerdFont)

### Community 31 - "Cache Populate Patch"
Cohesion: 1.0
Nodes (1): Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach

### Community 32 - "Cache Refresh Patch"
Cohesion: 1.0
Nodes (1): Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s

### Community 33 - "Cache Test Patches"
Cohesion: 1.0
Nodes (1): Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach

### Community 34 - "Scan Cache Pane Patch"
Cohesion: 1.0
Nodes (1): Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca

### Community 35 - "App Event System"
Cohesion: 1.0
Nodes (1): AppEvent

### Community 38 - "Terminal Integration"
Cohesion: 1.0
Nodes (2): Integrated Terminal Feature, Terminal Behavior Guide

### Community 39 - "Flyout Submenu"
Cohesion: 1.0
Nodes (2): ModalState::Menu flyout extension, Flyout Submenu (View→Themes)

### Community 40 - "Preview Buffer"
Cohesion: 1.0
Nodes (2): ViewBuffer, Preview Enhancements Design

## Knowledge Gaps
- **134 isolated node(s):** `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`, `MenuId` (+129 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `App State & Key Events`** (76 nodes): `key_event_to_string()`, `AppState`, `.active_menu()`, `.active_pane_title()`, `.active_workspace()`, `.active_workspace_index()`, `.active_workspace_mut()`, `.apply_config_reload()`, `.apply_rename_pattern()`, `.apply_view()`, `.archive_member_source()`, `.begin_open_editor()`, `.bookmarks()`, `.can_focus_preview_panel()`, `.config()`, `.config_path()`, `.copy_operation_for_source()`, `.deref()`, `.deref_mut()`, `.destructive_confirm()`, `.dialog()`, `.editor()`, `.editor_mut()`, `.file_finder()`, `.focus()`, `.focus_layer()`, `.image_picker()`, `.initial_commands()`, `.is_collision_open()`, `.is_dialog_open()`, `.is_editor_focused()`, `.is_editor_fullscreen()`, `.is_editor_loading()`, `.is_markdown_preview_focused()`, `.is_markdown_preview_visible()`, `.is_menu_open()`, `.is_palette_open()`, `.is_preview_focused()`, `.is_preview_panel_open()`, `.is_prompt_open()`, `.is_settings_open()`, `.is_settings_rebinding()`, `.is_terminal_fullscreen()`, `.left_pane()`, `.mark_drawn()`, `.mark_editor_saved()`, `.markdown_preview_scroll()`, `.menu_context()`, `.menu_items()`, `.menu_selection()`, `.needs_redraw()`, `.note_batch_settled()`, `.palette()`, `.pane_layout()`, `.pane_split_ratio()`, `.preview_command_due()`, `.preview_view()`, `.redraw_count()`, `.refresh_target_path_for_transfer()`, `.refresh_targets_for_prompt()`, `.right_pane()`, `.set_error_status()`, `.set_image_picker()`, `.set_needs_redraw()`, `.settings()`, `.settings_entries()`, `.settings_entries_for_tab()`, `.settings_mut()`, `.should_quit()`, `.ssh_connect()`, `.summarize_paths()`, `.sync_editor_menu_mode()`, `.theme()`, `.validate_rename_target()`, `.workspace_count()`, `.workspace_mut()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `FS Backend Interface`** (3 nodes): `CopyProgress`, `FsBackend`, `backend.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Cache Populate Patch`** (2 nodes): `patch_cache_populate.py`, `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Cache Refresh Patch`** (2 nodes): `patch_cache_refresh.py`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Cache Test Patches`** (2 nodes): `patch_cache_tests.py`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Scan Cache Pane Patch`** (2 nodes): `patch_scan_cache_pane.py`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `App Event System`** (2 nodes): `AppEvent`, `event.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Terminal Integration`** (2 nodes): `Integrated Terminal Feature`, `Terminal Behavior Guide`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Flyout Submenu`** (2 nodes): `ModalState::Menu flyout extension`, `Flyout Submenu (View→Themes)`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Preview Buffer`** (2 nodes): `ViewBuffer`, `Preview Enhancements Design`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ok()` connect `Application Orchestration` to `Core State Modules`, `Filesystem Backend Operations`, `State Reducers & Jobs`, `File Copy Operations`, `Pane Navigation & Selection`, `Editor Buffer & Cursor`, `App State & Key Events`, `Menu & Overlay UI`, `Preview & Archive`, `Fuzzy Find & Candidates`, `Pane Set & Focus`, `Version & Update`?**
  _High betweenness centrality (0.155) - this node is a cross-community bridge._
- **Why does `PaneState` connect `Pane Navigation & Selection` to `Filesystem Backend Operations`?**
  _High betweenness centrality (0.070) - this node is a cross-community bridge._
- **Why does `AppState` connect `App State & Key Events` to `Application Orchestration`, `State Reducers & Jobs`?**
  _High betweenness centrality (0.051) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `test_state()` (e.g. with `.default()` and `.resolve()`) actually correct?**
  _`test_state()` has 2 INFERRED edges - model-reasoned connections that need verification._
- **Are the 68 inferred relationships involving `ok()` (e.g. with `.bootstrap()` and `.run()`) actually correct?**
  _`ok()` has 68 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach` to the rest of the system?**
  _134 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Core State Modules` be split into smaller, more focused modules?**
  _Cohesion score 0.03 - nodes in this community are weakly interconnected._
# Graph Report - .  (2026-05-02)

## Corpus Check
- 33 files · ~50,000 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1747 nodes · 3419 edges · 51 communities detected
- Extraction: 85% EXTRACTED · 15% INFERRED · 0% AMBIGUOUS · INFERRED: 529 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_App State & Core Logic|App State & Core Logic]]
- [[_COMMUNITY_Filesystem & Backend|Filesystem & Backend]]
- [[_COMMUNITY_Actions & Key Events|Actions & Key Events]]
- [[_COMMUNITY_Jobs & Archive Workers|Jobs & Archive Workers]]
- [[_COMMUNITY_Editor Buffer|Editor Buffer]]
- [[_COMMUNITY_Config & Hook System|Config & Hook System]]
- [[_COMMUNITY_Pane Navigation|Pane Navigation]]
- [[_COMMUNITY_Docs & Architecture|Docs & Architecture]]
- [[_COMMUNITY_Filesystem Scan & Diff|Filesystem Scan & Diff]]
- [[_COMMUNITY_Overlay & Modal State|Overlay & Modal State]]
- [[_COMMUNITY_Preview & Archive|Preview & Archive]]
- [[_COMMUNITY_Git Integration|Git Integration]]
- [[_COMMUNITY_Markdown Renderer|Markdown Renderer]]
- [[_COMMUNITY_Icons & File Types|Icons & File Types]]
- [[_COMMUNITY_App Event Loop|App Event Loop]]
- [[_COMMUNITY_Module Group 15|Module Group 15]]
- [[_COMMUNITY_Module Group 16|Module Group 16]]
- [[_COMMUNITY_Module Group 17|Module Group 17]]
- [[_COMMUNITY_Module Group 18|Module Group 18]]
- [[_COMMUNITY_Module Group 19|Module Group 19]]
- [[_COMMUNITY_Module Group 20|Module Group 20]]
- [[_COMMUNITY_Module Group 21|Module Group 21]]
- [[_COMMUNITY_Module Group 22|Module Group 22]]
- [[_COMMUNITY_Module Group 23|Module Group 23]]
- [[_COMMUNITY_Module Group 24|Module Group 24]]
- [[_COMMUNITY_Module Group 25|Module Group 25]]
- [[_COMMUNITY_Module Group 26|Module Group 26]]
- [[_COMMUNITY_Module Group 27|Module Group 27]]
- [[_COMMUNITY_Module Group 28|Module Group 28]]
- [[_COMMUNITY_Module Group 29|Module Group 29]]
- [[_COMMUNITY_Module Group 30|Module Group 30]]
- [[_COMMUNITY_Module Group 31|Module Group 31]]
- [[_COMMUNITY_Module Group 32|Module Group 32]]
- [[_COMMUNITY_Module Group 34|Module Group 34]]
- [[_COMMUNITY_Module Group 35|Module Group 35]]
- [[_COMMUNITY_Module Group 36|Module Group 36]]
- [[_COMMUNITY_Module Group 37|Module Group 37]]
- [[_COMMUNITY_Module Group 38|Module Group 38]]
- [[_COMMUNITY_Module Group 39|Module Group 39]]
- [[_COMMUNITY_Module Group 40|Module Group 40]]
- [[_COMMUNITY_Module Group 41|Module Group 41]]
- [[_COMMUNITY_Module Group 42|Module Group 42]]
- [[_COMMUNITY_Module Group 43|Module Group 43]]
- [[_COMMUNITY_Module Group 44|Module Group 44]]
- [[_COMMUNITY_Module Group 49|Module Group 49]]
- [[_COMMUNITY_Module Group 50|Module Group 50]]
- [[_COMMUNITY_Module Group 63|Module Group 63]]
- [[_COMMUNITY_Module Group 64|Module Group 64]]
- [[_COMMUNITY_Module Group 65|Module Group 65]]
- [[_COMMUNITY_Module Group 67|Module Group 67]]
- [[_COMMUNITY_Module Group 68|Module Group 68]]

## God Nodes (most connected - your core abstractions)
1. `AppState` - 104 edges
2. `test_state()` - 96 edges
3. `EditorBuffer` - 49 edges
4. `PaneState` - 44 edges
5. `ok()` - 41 edges
6. `OverlayState` - 40 edges
7. `parse_markdown_lines()` - 35 edges
8. `route_key_event()` - 32 edges
9. `Action` - 26 edges
10. `ADR-0001 Core Architecture Decision` - 23 edges

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
- **Hook Execution Pipeline** — HookEvent, HookConfig, HookEnv, commands_for_event, Command_RunHook [INFERRED 0.90]
- **OnStart Hook Flow** — HookEvent, HookConfig, HookEnv, initial_commands, commands_for_event, Command_RunHook [INFERRED 0.85]
- **OnCd Hook Flow** — HookEvent, HookConfig, HookEnv, apply_job_result_commands, commands_for_event, Command_RunHook [INFERRED 0.85]
- **OnExit Hook Flow** — HookEvent, HookConfig, HookEnv, app_run_on_exit, commands_for_event, Command_RunHook [INFERRED 0.85]
- **First Run Wizard** — WizardState, WizardStep, render_first_run_wizard [EXTRACTED 1.00]
- **Hook Events with Environment Variables** — on_cd_event, on_open_event, on_start_event, on_exit_event, zeta_path_env, zeta_old_path_env, zeta_pane_env, zeta_version_env [EXTRACTED 1.00]
- **Four-Phase Development Roadmap** — phase_1_state_decomp, phase_2_ui_polish, phase_3_first_run, phase_4_shell_hooks [EXTRACTED 1.00]
- **First-Run Wizard Components** — first_run_wizard, theme_picker_step, live_theme_preview, annotated_config_gen, wizard_state [EXTRACTED 1.00]
- **Core Zeta Features** — dual_pane_browser, editor_module, workspaces_feature, ssh_sftp_feature, integrated_terminal, themes_feature, shell_hooks_feature, first_run_wizard [EXTRACTED 0.95]

## Communities

### Community 0 - "App State & Core Logic"
Cohesion: 0.02
Nodes (106): add_bookmark_persists_to_config(), AppState, batch_archive_extract_success_clears_marks_after_completed_result(), batch_full_failure_keeps_marks_and_reports_failed_status(), batch_full_success_clears_marks_and_sets_completed_status(), batch_move_success_clears_marks_after_completed_result(), batch_partial_failure_keeps_failed_marks_only(), batch_prompt_submit_does_not_clear_marks_at_dispatch() (+98 more)

### Community 1 - "Filesystem & Backend"
Cohesion: 0.03
Nodes (87): main(), F, LocalBackend, SftpBackend, ok(), create_file(), suggest_non_conflicting_path(), PlatformPty (+79 more)

### Community 2 - "Actions & Key Events"
Cohesion: 0.03
Nodes (94): Action, alt_menu_shortcuts_are_available(), CollisionPolicy, Command, editor_mode_prefers_text_entry(), editor_shift_number_keys_remain_text_input(), editor_shortcuts_remain_available(), FileOperation (+86 more)

### Community 3 - "Jobs & Archive Workers"
Cohesion: 0.03
Nodes (86): archive_worker_lists_zip_and_tar(), ArchiveListRequest, BackendRef, base64_encode(), build_hex_row(), connect_sftp(), describe_operation(), DirSizeRequest (+78 more)

### Community 4 - "Editor Buffer"
Cohesion: 0.05
Nodes (37): cursor_moves_between_lines(), Edit, EditorBuffer, EditorError, EditorRenderState, find_matches_empty_query_returns_nothing(), find_matches_is_case_insensitive(), find_matches_returns_all_occurrences() (+29 more)

### Community 5 - "Config & Hook System"
Cohesion: 0.04
Nodes (37): annotated_config_contains_comments(), annotated_config_contains_section_headers(), annotated_config_escapes_special_chars(), annotated_config_is_valid_toml(), annotated_config_theme_preset_round_trips(), AppConfig, assert_palette_ladder(), compiles_ctrl_key_binding() (+29 more)

### Community 6 - "Pane Navigation"
Cohesion: 0.05
Nodes (27): clamps_selection_at_zero(), clear_marks_removes_all(), cycle_sort_mode_wraps_around(), dir_first(), empty_pane(), filter_active_hides_non_matching_entries(), filter_empty_query_shows_all_entries(), filter_is_case_insensitive() (+19 more)

### Community 7 - "Docs & Architecture"
Cohesion: 0.05
Nodes (63): ADR-0001 Core Architecture Decision, AGENTS.md Project Conventions, Changelog, Global Command Palette (Ctrl+P), Confirmation Modals for Destructive Actions, Dual-Pane Layout, EditorState (Editor Sub-State), ZetaError Context Propagation (+55 more)

### Community 8 - "Filesystem Scan & Diff"
Cohesion: 0.06
Nodes (46): compute_scan_diff(), detects_added_entry(), detects_modified_entry_by_mtime(), detects_modified_entry_by_size(), detects_removed_entry(), empty_diff_when_no_changes(), entry(), ScanDiff (+38 more)

### Community 9 - "Overlay & Modal State"
Cohesion: 0.07
Nodes (21): close_all_removes_modal(), enter_flyout_not_on_trigger_switches_tab(), enter_flyout_on_trigger_activates_flyout_item(), exit_flyout_when_closed_switches_prev_tab(), exit_flyout_when_open_collapses_flyout(), flyout_trigger(), menu_activate_emits_dispatch_action(), menu_activate_on_flyout_item_dispatches_action() (+13 more)

### Community 10 - "Preview & Archive"
Cohesion: 0.06
Nodes (25): archive_listing_is_detected(), ArchiveEntry, ArchiveFormat, ArchiveListing, from_plain_builds_correct_total(), hex_dump_is_detected(), HexDumpData, HexRow (+17 more)

### Community 11 - "Git Integration"
Cohesion: 0.08
Nodes (43): classify(), current_branch(), detect_repo(), DiffLine, DiffLineKind, fetch_diff_files(), fetch_file_diff(), fetch_status() (+35 more)

### Community 12 - "Markdown Renderer"
Cohesion: 0.08
Nodes (52): blank_line_produces_empty_line(), blockquote_uses_bar_prefix(), bold_italic_combined_applies_both_modifiers(), bullet_list_uses_bullet_char(), default_palette(), fence_lang(), fenced_block_shows_language_tag(), fenced_code_block_collects_inner_lines() (+44 more)

### Community 13 - "Icons & File Types"
Cohesion: 0.05
Nodes (26): icon_for_entry(), icon_for_kind(), nerdfont_icon(), unicode_icon(), days_to_ymd(), display_width(), format_entry_meta(), format_icon_slot() (+18 more)

### Community 14 - "App Event Loop"
Cohesion: 0.06
Nodes (43): AppEvent::Mouse Variant, ArchiveWorker (6th background worker), Bookmarks in AppConfig (Vec<PathBuf>), compute_diff() — pure directory comparison, DiffStatus Enum (LeftOnly/RightOnly/Same/Different), EditorBuffer, FinderWorker (5th background worker), FocusLayer Enum (+35 more)

### Community 15 - "Module Group 15"
Cohesion: 0.07
Nodes (35): Annotated Config Generation, apply_view() Monolith, Command::RunHook Variant, config.toml, Contextual Hints Bar, Dual-Pane Browser, Embedded Text Editor, First-Run Wizard (+27 more)

### Community 16 - "Module Group 16"
Cohesion: 0.11
Nodes (15): bail(), _c(), Candidate, cargo(), compute_candidates(), git(), header(), main() (+7 more)

### Community 17 - "Module Group 17"
Cohesion: 0.09
Nodes (27): build_row_spans(), char_highlight_bg(), CodeViewRenderArgs, render_code_view(), SearchHighlight, SelectionHighlight, styled_span(), editor_highlighted_render_state() (+19 more)

### Community 18 - "Module Group 18"
Cohesion: 0.12
Nodes (23): copy_path(), copy_path_recursive(), copy_path_rejects_existing_destination(), copy_path_with_progress(), copy_path_with_progress_reports_completed_entries(), count_path_entries(), count_path_entries_counts_directories_and_files(), create_directory() (+15 more)

### Community 19 - "Module Group 19"
Cohesion: 0.1
Nodes (7): CollisionState, DestructiveAction, DestructiveConfirmState, DialogState, prompt_base_path(), PromptKind, PromptState

### Community 20 - "Module Group 20"
Cohesion: 0.18
Nodes (13): close_editor_when_dirty_keeps_buffer(), close_editor_when_not_dirty_removes_buffer(), cut_removes_selected_text(), discard_closes_buffer(), EditorState, open_markdown_file_enables_live_preview(), redo_reapplies_undone_insert(), select_all_covers_entire_buffer() (+5 more)

### Community 21 - "Module Group 21"
Cohesion: 0.12
Nodes (20): Zeta Development Roadmap, App Screenshot — Workspaces & Editor, Architecture Remediation & Feature Foundation Design, AppState Decomposition, FileOperationIdentity, IconMode, WorkspaceState, Custom Icon Font Design (+12 more)

### Community 22 - "Module Group 22"
Cohesion: 0.16
Nodes (11): command_palette_header_is_muted_and_bold(), command_palette_hint_uses_key_hint_fg(), command_palette_selected_entry_uses_selection_bg(), command_palette_unselected_entry_uses_text_primary(), editor_render_state_tracks_viewport(), elevated_surface_uses_tools_bg(), overlay_title_is_bold_and_mnemonic_fg(), pane_chrome_focused_uses_border_focus_color() (+3 more)

### Community 23 - "Module Group 23"
Cohesion: 0.19
Nodes (11): compute_diff(), compute_diff_different_size(), compute_diff_directories_match_by_name(), compute_diff_left_only(), compute_diff_right_only(), compute_diff_same_entry(), compute_diff_symmetric_count(), DiffStatus (+3 more)

### Community 24 - "Module Group 24"
Cohesion: 0.21
Nodes (14): all_entries(), category_order(), filter_case_insensitive(), filter_empty_query_returns_all(), filter_entries(), filter_subsequence_matches_label(), filter_subsequence_no_match_returns_empty(), is_subsequence() (+6 more)

### Community 25 - "Module Group 25"
Cohesion: 0.27
Nodes (15): AppConfig.hooks Field, RunHook Command Variant, Hook Configuration, Hook Runtime Context, Hook Event Enum, Wizard State Machine, Wizard Step Enum, App Exit Hook Firing (+7 more)

### Community 26 - "Module Group 26"
Cohesion: 0.23
Nodes (11): menu_items_for(), menu_tabs(), MenuContext, MenuTab, navigate_menu_starts_with_workspace_switch_items(), context_badge_spans(), menu_spans(), render_menu_bar() (+3 more)

### Community 27 - "Module Group 27"
Cohesion: 0.15
Nodes (5): KeymapField, SettingsEntry, SettingsField, SettingsState, SettingsTab

### Community 28 - "Module Group 28"
Cohesion: 0.2
Nodes (3): legacy_session_fields_migrate_into_first_workspace(), SessionState, WorkspaceSessionState

### Community 29 - "Module Group 29"
Cohesion: 0.29
Nodes (5): parse_hunk_header(), render_diff_content(), render_diff_file_list(), render_git_diff_view(), render_line_with_gutter()

### Community 30 - "Module Group 30"
Cohesion: 0.36
Nodes (8): e2e_enter_confirmation(), e2e_escape_clears_overlay(), e2e_filter_input(), e2e_home_end_navigation(), e2e_navigate_down_in_pane(), e2e_quit_gracefully(), e2e_startup_shows_panes(), e2e_switch_pane()

### Community 31 - "Module Group 31"
Cohesion: 0.24
Nodes (6): is_newer_version(), parse_version_tag(), Release, UpdateChecker, UpdateError, UpdateStatus

### Community 32 - "Module Group 32"
Cohesion: 0.22
Nodes (5): HostKeyFingerprints, SshAuthMethod, SshConnectionState, SshDialogField, SshErrorKind

### Community 34 - "Module Group 34"
Cohesion: 0.33
Nodes (1): LayoutCache

### Community 35 - "Module Group 35"
Cohesion: 0.67
Nodes (2): CopyProgress, FsBackend

### Community 36 - "Module Group 36"
Cohesion: 0.67
Nodes (1): BookmarksState

### Community 37 - "Module Group 37"
Cohesion: 0.67
Nodes (3): StatusZones (zoned status bar), ThemePalette (Catppuccin Mocha + accent tokens), UI/UX Revamp (Catppuccin + NerdFont)

### Community 38 - "Module Group 38"
Cohesion: 0.67
Nodes (3): FileOperationIdentity / FileOperationKind, File Operation Identity Hardening, Operation Safety Hardening

### Community 39 - "Module Group 39"
Cohesion: 1.0
Nodes (3): UpdateChecker (GitHub API + self-update), Update Checks & Self-Update, ureq crate v2.9 (HTTP client)

### Community 40 - "Module Group 40"
Cohesion: 1.0
Nodes (1): Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach

### Community 41 - "Module Group 41"
Cohesion: 1.0
Nodes (1): Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s

### Community 42 - "Module Group 42"
Cohesion: 1.0
Nodes (1): Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach

### Community 43 - "Module Group 43"
Cohesion: 1.0
Nodes (1): Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca

### Community 44 - "Module Group 44"
Cohesion: 1.0
Nodes (1): AppEvent

### Community 49 - "Module Group 49"
Cohesion: 1.0
Nodes (2): ModalState::Menu flyout extension, Flyout Submenu (View→Themes)

### Community 50 - "Module Group 50"
Cohesion: 1.0
Nodes (2): ViewBuffer, Preview Enhancements Design

### Community 63 - "Module Group 63"
Cohesion: 1.0
Nodes (1): Performance Baseline

### Community 64 - "Module Group 64"
Cohesion: 1.0
Nodes (1): Release Flow Documentation

### Community 65 - "Module Group 65"
Cohesion: 1.0
Nodes (1): Terminal Behavior Guide

### Community 67 - "Module Group 67"
Cohesion: 1.0
Nodes (1): Job Result Sender

### Community 68 - "Module Group 68"
Cohesion: 1.0
Nodes (1): Wizard State Module

## Knowledge Gaps
- **157 isolated node(s):** `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`, `EditorRenderState` (+152 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Module Group 34`** (6 nodes): `layout_cache.rs`, `LayoutCache`, `rect_contains()`, `rect_contains_returns_false_for_border_outside()`, `rect_contains_returns_true_for_inner_cell()`, `rect_contains_zero_size_rect_never_matches()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 35`** (3 nodes): `CopyProgress`, `FsBackend`, `backend.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 36`** (3 nodes): `bookmarks.rs`, `BookmarksState`, `.new()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 40`** (2 nodes): `patch_cache_populate.py`, `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 41`** (2 nodes): `patch_cache_refresh.py`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 42`** (2 nodes): `patch_cache_tests.py`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 43`** (2 nodes): `patch_scan_cache_pane.py`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 44`** (2 nodes): `AppEvent`, `event.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 49`** (2 nodes): `ModalState::Menu flyout extension`, `Flyout Submenu (View→Themes)`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 50`** (2 nodes): `ViewBuffer`, `Preview Enhancements Design`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 63`** (1 nodes): `Performance Baseline`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 64`** (1 nodes): `Release Flow Documentation`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 65`** (1 nodes): `Terminal Behavior Guide`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 67`** (1 nodes): `Job Result Sender`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Module Group 68`** (1 nodes): `Wizard State Module`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppState` connect `App State & Core Logic` to `Actions & Key Events`, `Jobs & Archive Workers`?**
  _High betweenness centrality (0.035) - this node is a cross-community bridge._
- **Why does `test_state()` connect `App State & Core Logic` to `Actions & Key Events`, `Config & Hook System`?**
  _High betweenness centrality (0.023) - this node is a cross-community bridge._
- **Why does `ok()` connect `Filesystem & Backend` to `Editor Buffer`, `Preview & Archive`, `Module Group 16`, `Module Group 18`, `Module Group 20`, `Module Group 30`?**
  _High betweenness centrality (0.022) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `test_state()` (e.g. with `.resolve()` and `.default()`) actually correct?**
  _`test_state()` has 2 INFERRED edges - model-reasoned connections that need verification._
- **Are the 38 inferred relationships involving `ok()` (e.g. with `.open()` and `.save()`) actually correct?**
  _`ok()` has 38 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach` to the rest of the system?**
  _157 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `App State & Core Logic` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
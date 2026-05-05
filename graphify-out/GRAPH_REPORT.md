# Graph Report - .  (2026-05-03)

## Corpus Check
- 0 files · ~999,999 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 2172 nodes · 4536 edges · 63 communities detected
- Extraction: 74% EXTRACTED · 26% INFERRED · 0% AMBIGUOUS · INFERRED: 1163 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
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
- [[_COMMUNITY_Community 46|Community 46]]
- [[_COMMUNITY_Community 47|Community 47]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 71|Community 71]]
- [[_COMMUNITY_Community 72|Community 72]]
- [[_COMMUNITY_Community 73|Community 73]]
- [[_COMMUNITY_Community 75|Community 75]]
- [[_COMMUNITY_Community 76|Community 76]]
- [[_COMMUNITY_Community 77|Community 77]]
- [[_COMMUNITY_Community 78|Community 78]]
- [[_COMMUNITY_Community 79|Community 79]]
- [[_COMMUNITY_Community 80|Community 80]]
- [[_COMMUNITY_Community 81|Community 81]]
- [[_COMMUNITY_Community 82|Community 82]]
- [[_COMMUNITY_Community 83|Community 83]]
- [[_COMMUNITY_Community 84|Community 84]]

## God Nodes (most connected - your core abstractions)
1. `test_state()` - 111 edges
2. `AppState` - 105 edges
3. `ok()` - 81 edges
4. `EditorBuffer` - 49 edges
5. `PaneState` - 44 edges
6. `OverlayState` - 40 edges
7. `render()` - 37 edges
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
- **Zeta Technology Stack** — agents_zeta, agents_rust, agents_crossterm, agents_ratatui, agents_ropey, agents_serde_toml [EXTRACTED 1.00]
- **Zeta Module Architecture** — agents_mod_app, agents_mod_state, agents_mod_action, agents_mod_ui, agents_mod_pane, agents_mod_fs, agents_mod_jobs, agents_mod_preview, agents_mod_editor, agents_mod_config [EXTRACTED 1.00]
- **SSH Security and Authentication Stack** — impl_summary_host_key_fingerprints, impl_summary_ssh_connection_state, ssh_sftp_host_key_verification, wave7b_auth_priority [EXTRACTED 0.92]
- **Zeta Core Features** — index_feature_dual_pane, index_feature_embedded_editor, index_feature_integrated_terminal, index_feature_ssh_sftp, index_feature_diff_mode, index_feature_workspaces, index_feature_markdown_preview, index_feature_customizable [EXTRACTED 1.00]
- **Multi-Worker Background Job Architecture** — plan_wave1c_scan_worker, plan_wave1c_file_op_worker, plan_wave1c_preview_worker, plan_wave1c_worker_channels [EXTRACTED 0.95]
- **Wave 1 Parallel Refactoring Plans** — plan_wave1a_appstate_decomp, plan_wave1b_ui_split, plan_wave1c_multiworker [EXTRACTED 0.95]
- **Directory Watch System** — wave5a_watcher_worker, wave5a_notify_crate, wave5a_directory_changed [EXTRACTED 1.00]
- **Filesystem Abstraction Layer** — wave7a_fs_backend, wave7a_local_backend, wave7a_sftp_backend [EXTRACTED 1.00]
- **Per-Workspace Runtime State** — workspaces_workspace_state, wave8a_terminal_state, wave3a_editor_buffer [EXTRACTED 0.90]
- **All Completed Development Waves** — roadmap_wave_1a, roadmap_wave_1b, roadmap_wave_1c, roadmap_wave_2a, roadmap_wave_2b, roadmap_wave_3a, roadmap_wave_4a, roadmap_wave_4b, roadmap_wave_4c, roadmap_wave_4d, roadmap_wave_5a, roadmap_wave_5b, roadmap_wave_5c, roadmap_wave_6a, roadmap_wave_6b, roadmap_wave_7a, roadmap_wave_7b, roadmap_wave_8a [EXTRACTED 1.00]
- **Editor-Related Features and Bindings** — roadmap_wave_3a, roadmap_wave_4b, roadmap_wave_4c, roadmap_wave_5a, keybindings_editor [INFERRED 0.80]
- **Git-Related Features and Bindings** — roadmap_wave_4a, roadmap_git_diff_viewer, keybindings_git_diff, keybindings_panels_views [INFERRED 0.80]
- **AppState Decomposition into Sub-States** — architecture_remediation_spec, app_state, workspace_state, overlay_state [INFERRED 0.90]
- **File Operation Correctness System** — operation_safety_spec, file_operation_identity_spec, pending_batch_operation, file_operation_identity_type [EXTRACTED 1.00]
- **Preview Enhancement Feature Set** — preview_enhancements_plan, preview_enhancements_spec, ratatui_image_crate, archive_listing, hex_dump_data [EXTRACTED 1.00]
- **Shell Hook Lifecycle Events** — shellhooks_on_start, shellhooks_on_exit, shellhooks_on_cd, shellhooks_on_open [EXTRACTED 1.00]
- **Shell Hook Environment Variables** — shellhooks_zeta_path, shellhooks_zeta_old_path, shellhooks_zeta_pane, shellhooks_zeta_version [EXTRACTED 1.00]

## Communities

### Community 0 - "Community 0"
Cohesion: 0.02
Nodes (176): alt_menu_shortcuts_are_available(), editor_mode_prefers_text_entry(), editor_shift_number_keys_remain_text_input(), editor_shortcuts_remain_available(), help_shortcuts_are_available(), movement_keys_remain_available(), workspace_shortcuts_switch_workspaces(), bookmarks_layer_routes_enter_to_confirm_selection() (+168 more)

### Community 1 - "Community 1"
Cohesion: 0.02
Nodes (120): add_bookmark_persists_to_config(), apply_update_is_noop_when_no_update(), apply_update_opens_prompt_when_update_available(), AppState, batch_archive_extract_success_clears_marks_after_completed_result(), batch_full_failure_keeps_marks_and_reports_failed_status(), batch_full_success_clears_marks_and_sets_completed_status(), batch_move_success_clears_marks_after_completed_result() (+112 more)

### Community 2 - "Community 2"
Cohesion: 0.02
Nodes (179): F, CollisionPolicy, Command, FileOperation, from_palette_key_event_handles_esc(), from_pane_key_event_handles_quit(), git_diff_content_ctrl_d_returns_none(), git_diff_content_down_returns_scroll_down() (+171 more)

### Community 3 - "Community 3"
Cohesion: 0.02
Nodes (140): AppEvent::Mouse Variant, AppState, Architecture Remediation and Feature Foundation Design, ArchiveListing, ArchiveWorker (6th background worker), Bookmarks in AppConfig (Vec<PathBuf>), compute_diff() — pure directory comparison, DiffStatus Enum (LeftOnly/RightOnly/Same/Different) (+132 more)

### Community 4 - "Community 4"
Cohesion: 0.03
Nodes (57): main(), LocalBackend, SftpBackend, ok(), App, relaunch_self(), run_update_and_restart(), TerminalSession (+49 more)

### Community 5 - "Community 5"
Cohesion: 0.04
Nodes (42): annotated_config_contains_comments(), annotated_config_contains_section_headers(), annotated_config_escapes_special_chars(), annotated_config_is_valid_toml(), annotated_config_theme_preset_round_trips(), AppConfig, assert_palette_ladder(), compiles_ctrl_key_binding() (+34 more)

### Community 6 - "Community 6"
Cohesion: 0.03
Nodes (76): ADR-0001 Core Architecture, crossbeam-channel crate, crossterm crate, Event-Action-Reducer Flow, Modular Monolith Architecture, action module, app module, config module (+68 more)

### Community 7 - "Community 7"
Cohesion: 0.05
Nodes (27): clamps_selection_at_zero(), clear_marks_removes_all(), cycle_sort_mode_wraps_around(), dir_first(), empty_pane(), filter_active_hides_non_matching_entries(), filter_empty_query_shows_all_entries(), filter_is_case_insensitive() (+19 more)

### Community 8 - "Community 8"
Cohesion: 0.05
Nodes (26): menu_items_for(), MenuContext, MenuTab, navigate_menu_starts_with_workspace_switch_items(), close_all_removes_modal(), ContextMenuItem, enter_flyout_not_on_trigger_switches_tab(), enter_flyout_on_trigger_activates_flyout_item() (+18 more)

### Community 9 - "Community 9"
Cohesion: 0.05
Nodes (63): ADR-0001 Core Architecture Decision, AGENTS.md Project Conventions, Changelog, Global Command Palette (Ctrl+P), Confirmation Modals for Destructive Actions, Dual-Pane Layout, EditorState (Editor Sub-State), ZetaError Context Propagation (+55 more)

### Community 10 - "Community 10"
Cohesion: 0.06
Nodes (29): looks_like_binary(), load_image_preview(), load_preview_content(), load_preview_from_bytes(), archive_listing_is_detected(), ArchiveEntry, ArchiveFormat, ArchiveListing (+21 more)

### Community 11 - "Community 11"
Cohesion: 0.07
Nodes (43): classify(), current_branch(), detect_repo(), DiffLine, DiffLineKind, fetch_diff_files(), fetch_file_diff(), fetch_status() (+35 more)

### Community 12 - "Community 12"
Cohesion: 0.08
Nodes (48): blank_line_produces_empty_line(), blockquote_uses_bar_prefix(), bold_italic_combined_applies_both_modifiers(), bullet_list_uses_bullet_char(), default_palette(), fence_lang(), fenced_block_shows_language_tag(), fenced_code_block_collects_inner_lines() (+40 more)

### Community 13 - "Community 13"
Cohesion: 0.05
Nodes (49): config.toml Keymap Configuration, Key Bindings Documentation, Editor Key Bindings, File Operations Key Bindings, File Pane Navigation Key Bindings, Git Diff Viewer Key Bindings, Global Key Bindings, Panels and Views Key Bindings (+41 more)

### Community 14 - "Community 14"
Cohesion: 0.09
Nodes (7): EditorBuffer, highlight_text(), normalize_preview_text(), syntax_set(), theme_set(), to_ratatui_color(), to_ratatui_modifier()

### Community 15 - "Community 15"
Cohesion: 0.06
Nodes (44): anyhow, Cargo, crossbeam-channel, crossterm, flume, action module, app module, config module (+36 more)

### Community 16 - "Community 16"
Cohesion: 0.06
Nodes (18): FocusLayer, MenuItem, MessageKind, ModalKind, PaneFocus, PaneLayout, status_message_error_constructor(), status_message_warning_constructor() (+10 more)

### Community 17 - "Community 17"
Cohesion: 0.09
Nodes (39): AppConfig Struct, apply_view() Monolith Method, Batch File Operations, Command::RunHook Variant, Config Hot-Reload Feature, Dual-Pane Browser, Git Status Indicators, HookConfig Struct (+31 more)

### Community 18 - "Community 18"
Cohesion: 0.07
Nodes (35): Annotated Config Generation, apply_view() Monolith, Command::RunHook Variant, config.toml, Contextual Hints Bar, Dual-Pane Browser, Embedded Text Editor, First-Run Wizard (+27 more)

### Community 19 - "Community 19"
Cohesion: 0.11
Nodes (15): bail(), _c(), Candidate, cargo(), compute_candidates(), git(), header(), main() (+7 more)

### Community 20 - "Community 20"
Cohesion: 0.08
Nodes (32): Cheatsheet Test Suite, Bookmarks Overlay (Alt+N then k), Cheatsheet Overlay (? key / Quick Reference), Command Palette Overlay (Shift+P), Context Menu (Shift+F10), Embedded Editor Mode (F4), File Finder Overlay (Ctrl+P), Dual-Pane File Manager UI (+24 more)

### Community 21 - "Community 21"
Cohesion: 0.1
Nodes (21): icon_for_entry(), icon_for_kind(), nerdfont_icon(), unicode_icon(), icon_slot_ascii_returns_icon_only(), icon_slot_unicode_appends_two_spaces(), days_to_ymd(), display_width() (+13 more)

### Community 22 - "Community 22"
Cohesion: 0.15
Nodes (3): Action, KeyBinding, route_key_event()

### Community 23 - "Community 23"
Cohesion: 0.09
Nodes (8): CollisionState, DestructiveAction, DestructiveConfirmState, DialogState, prompt_base_path(), PromptKind, PromptState, resolve_prompt_target()

### Community 24 - "Community 24"
Cohesion: 0.18
Nodes (19): route_menu_bar_click(), route_mouse_event(), route_mouse_left_click_on_dialog_closes_it(), route_mouse_left_click_on_file_menu_opens_file_menu(), route_mouse_left_click_on_pane_produces_action(), route_mouse_left_click_on_right_pane_produces_right_pane_click(), route_mouse_left_click_on_workspace_pill_2_switches_workspace(), route_mouse_left_click_on_workspace_pill_4_switches_workspace() (+11 more)

### Community 25 - "Community 25"
Cohesion: 0.11
Nodes (2): glob_match(), matches()

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
Cohesion: 0.22
Nodes (8): focus_next_pane_cycles_left_to_right(), focus_next_pane_cycles_right_to_left(), inactive_pane_returns_opposite_of_focus(), make_state(), PaneSetState, refresh_with_fresh_cache_skips_scan(), refresh_with_no_cache_queues_scan(), refresh_with_stale_mtime_queues_scan()

### Community 30 - "Community 30"
Cohesion: 0.27
Nodes (15): AppConfig.hooks Field, RunHook Command Variant, Hook Configuration, Hook Runtime Context, Hook Event Enum, Wizard State Machine, Wizard Step Enum, App Exit Hook Firing (+7 more)

### Community 31 - "Community 31"
Cohesion: 0.15
Nodes (5): KeymapField, SettingsEntry, SettingsField, SettingsState, SettingsTab

### Community 32 - "Community 32"
Cohesion: 0.31
Nodes (8): compute_scan_diff(), detects_added_entry(), detects_modified_entry_by_mtime(), detects_modified_entry_by_size(), detects_removed_entry(), empty_diff_when_no_changes(), entry(), ScanDiff

### Community 33 - "Community 33"
Cohesion: 0.24
Nodes (6): is_newer_version(), parse_version_tag(), Release, UpdateChecker, UpdateError, UpdateStatus

### Community 34 - "Community 34"
Cohesion: 0.22
Nodes (5): HostKeyFingerprints, SshAuthMethod, SshConnectionState, SshDialogField, SshErrorKind

### Community 35 - "Community 35"
Cohesion: 0.39
Nodes (5): ascii_match(), case_insensitive_ascii(), split_at_match(), unicode_expanding_lowercase(), unicode_non_expanding()

### Community 36 - "Community 36"
Cohesion: 0.38
Nodes (7): Custom Icon Font Design Spec, IconMode Enum, Neo-Commander UI/UX Design Spec, StatusZones (zoned status bar), Theme Palette (Catppuccin Mocha), UI/UX Revamp (Catppuccin + NerdFont), UI/UX Revamp Design Spec

### Community 37 - "Community 37"
Cohesion: 0.4
Nodes (5): CatppuccinMocha Theme Preset, modal_halo Colour Token, UI/UX Revamp Plan, StatusZones (Zoned Status Bar), ThemePalette Expansion (Accent Tokens)

### Community 38 - "Community 38"
Cohesion: 0.5
Nodes (4): AppEvent::Mouse Variant, EnableMouseCapture / DisableMouseCapture, Wave 2B: Full Mouse Support Plan, route_mouse_event Function

### Community 39 - "Community 39"
Cohesion: 0.83
Nodes (4): File Operation Identity Hardening Design, FileOperationIdentity, Operation Safety Hardening Design, PendingBatchOperation

### Community 40 - "Community 40"
Cohesion: 0.5
Nodes (4): File Marks (Space key, › * indicator), Pane Filter (/ key, inline filter bar), Sort Cycling (s key), Filter and Sort Test Suite

### Community 41 - "Community 41"
Cohesion: 0.67
Nodes (2): CopyProgress, FsBackend

### Community 42 - "Community 42"
Cohesion: 0.67
Nodes (3): FileOperationIdentity / FileOperationKind, File Operation Identity Hardening, Operation Safety Hardening

### Community 43 - "Community 43"
Cohesion: 0.67
Nodes (3): MenuEnterFlyout / MenuExitFlyout Actions, ModalState::Menu flyout Extension, Flyout Submenu Plan (View → Themes)

### Community 44 - "Community 44"
Cohesion: 1.0
Nodes (1): Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach

### Community 45 - "Community 45"
Cohesion: 1.0
Nodes (1): Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s

### Community 46 - "Community 46"
Cohesion: 1.0
Nodes (1): Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach

### Community 47 - "Community 47"
Cohesion: 1.0
Nodes (1): Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca

### Community 48 - "Community 48"
Cohesion: 1.0
Nodes (1): AppEvent

### Community 51 - "Community 51"
Cohesion: 1.0
Nodes (2): ViewBuffer, Preview Enhancements Design

### Community 71 - "Community 71"
Cohesion: 1.0
Nodes (1): Performance Baseline

### Community 72 - "Community 72"
Cohesion: 1.0
Nodes (1): Release Flow Documentation

### Community 73 - "Community 73"
Cohesion: 1.0
Nodes (1): Terminal Behavior Guide

### Community 75 - "Community 75"
Cohesion: 1.0
Nodes (1): Job Result Sender

### Community 76 - "Community 76"
Cohesion: 1.0
Nodes (1): Wizard State Module

### Community 77 - "Community 77"
Cohesion: 1.0
Nodes (1): Wave 2A: Input Routing (FocusLayer)

### Community 78 - "Community 78"
Cohesion: 1.0
Nodes (1): Wave 2B: Mouse Support

### Community 79 - "Community 79"
Cohesion: 1.0
Nodes (1): Wave 4D: Quick Filter + Fuzzy File Find

### Community 80 - "Community 80"
Cohesion: 1.0
Nodes (1): Wave 5A: Find & Replace + Directory Watcher

### Community 81 - "Community 81"
Cohesion: 1.0
Nodes (1): Wave 5B: Bookmarks + Trash

### Community 82 - "Community 82"
Cohesion: 1.0
Nodes (1): Wave 6A: Archive Browsing

### Community 83 - "Community 83"
Cohesion: 1.0
Nodes (1): Live Clock Status Bar

### Community 84 - "Community 84"
Cohesion: 1.0
Nodes (1): ThemePalette v2

## Knowledge Gaps
- **288 isolated node(s):** `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`, `EditorRenderState` (+283 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 25`** (20 nodes): `glob_match.rs`, `double_star_acts_like_single_star()`, `empty_pattern_matches_everything_via_substring()`, `exact_glob_match()`, `glob_is_case_insensitive()`, `glob_match()`, `matches()`, `mixed_wildcards()`, `negation_inverts_match()`, `negation_with_question_mark()`, `pattern_longer_than_name_does_not_match()`, `plain_query_is_case_insensitive()`, `plain_query_is_substring_match()`, `question_does_not_match_two_chars()`, `question_does_not_match_zero_chars()`, `question_matches_exactly_one_char()`, `star_does_not_match_wrong_extension()`, `star_matches_any_prefix()`, `star_matches_any_suffix()`, `star_only_matches_everything()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 41`** (3 nodes): `CopyProgress`, `FsBackend`, `backend.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 44`** (2 nodes): `patch_cache_populate.py`, `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 45`** (2 nodes): `patch_cache_refresh.py`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 46`** (2 nodes): `patch_cache_tests.py`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 47`** (2 nodes): `patch_scan_cache_pane.py`, `Patch src/pane.rs:   1. Add `use std::time::SystemTime;` to imports   2. Add Sca`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 48`** (2 nodes): `AppEvent`, `event.rs`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 51`** (2 nodes): `ViewBuffer`, `Preview Enhancements Design`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 71`** (1 nodes): `Performance Baseline`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 72`** (1 nodes): `Release Flow Documentation`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 73`** (1 nodes): `Terminal Behavior Guide`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 75`** (1 nodes): `Job Result Sender`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 76`** (1 nodes): `Wizard State Module`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 77`** (1 nodes): `Wave 2A: Input Routing (FocusLayer)`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 78`** (1 nodes): `Wave 2B: Mouse Support`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 79`** (1 nodes): `Wave 4D: Quick Filter + Fuzzy File Find`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 80`** (1 nodes): `Wave 5A: Find & Replace + Directory Watcher`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 81`** (1 nodes): `Wave 5B: Bookmarks + Trash`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 82`** (1 nodes): `Wave 6A: Archive Browsing`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 83`** (1 nodes): `Live Clock Status Bar`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.
- **Thin community `Community 84`** (1 nodes): `ThemePalette v2`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `ok()` connect `Community 4` to `Community 0`, `Community 1`, `Community 2`, `Community 33`, `Community 5`, `Community 8`, `Community 10`, `Community 11`, `Community 19`, `Community 29`?**
  _High betweenness centrality (0.108) - this node is a cross-community bridge._
- **Why does `LayoutCache` connect `Community 3` to `Community 24`?**
  _High betweenness centrality (0.087) - this node is a cross-community bridge._
- **Are the 3 inferred relationships involving `test_state()` (e.g. with `.resolve()` and `.default()`) actually correct?**
  _`test_state()` has 3 INFERRED edges - model-reasoned connections that need verification._
- **Are the 78 inferred relationships involving `ok()` (e.g. with `.open()` and `.save()`) actually correct?**
  _`ok()` has 78 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Patch src/state/mod.rs inside the DirectoryScanned handler:   Populate scan_cach`, `Patch src/state/pane_set.rs:   - Action::Refresh: check scan_cache.is_fresh(); s`, `Add three ScanCache tests to src/state/pane_set.rs:   1. refresh_with_fresh_cach` to the rest of the system?**
  _288 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.02 - nodes in this community are weakly interconnected._
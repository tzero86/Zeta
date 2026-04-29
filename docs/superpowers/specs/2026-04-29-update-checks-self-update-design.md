# Update Checks and Self-Update Feature Design

**Date:** 2026-04-29  
**Status:** Design phase  
**Author:** Copilot CLI  

---

## Executive Summary

Add automatic update checking and self-update capabilities to Zeta. The feature will:

1. **Check for updates** on every app startup and on-demand via the Help menu
2. **Notify users subtly** with a pulsing indicator in the status bar + Help menu entry
3. **Download and install** updates on user confirmation (Zeta binary replacement via `cargo install`)
4. **Require manual restart** to apply the update (no auto-restart)

The implementation is non-blocking, network-resilient, and respects user attention.

---

## Goals and Success Criteria

### Goals
- Users are always aware when a new version is available
- Updates are accessible without leaving Zeta
- Minimal visual disruption; users can ignore the notification if they wish
- Safe operation: no silent installs, version verification before download

### Success Criteria
- Startup check completes in < 500ms, with no blocking of app initialization
- Failed network requests fail gracefully (log silently, don't disrupt UI)
- Users can update in 2-3 clicks (status bar hint → dialog → confirm)
- Binary replacement works for `cargo install` installations

---

## Feature Overview

### User Journey: Startup Check

1. User launches Zeta
2. App initializes normally (no blocking)
3. Background task checks GitHub API for latest release
4. If new version available:
   - Status bar shows pulsing red "● Update" indicator next to clock
   - Help menu shows "🔴 Check for Updates → v0.5.0 available" entry
5. User can:
   - Click status bar hint to open update dialog immediately
   - Open Help menu anytime to see version info
   - Ignore notification (indicator persists until next restart)

### User Journey: Update Confirmation

1. User clicks status bar hint or Help menu entry
2. Dialog opens showing:
   - Current version (e.g., v0.4.5)
   - Available version (e.g., v0.5.0)
   - Brief release notes (if available in GitHub API response)
   - "Download & Install" and "Later" buttons
3. User clicks "Download & Install"
4. Download progress shown in dialog (non-blocking background task)
5. After download completes:
   - Dialog shows "Update downloaded. Restart Zeta to apply."
   - "Restart Now" and "Later" buttons appear
6. User clicks "Restart Now" or manually restarts

### Manual Check

- User opens Help menu and selects "Check for Updates" (or command palette)
- Trigggers same check as startup, shows result in dialog even if already checked

---

## Architecture

### New Modules

#### `src/update.rs`
- **UpdateChecker** struct: queries GitHub API, parses JSON response
  - `check_latest_release()` → Result<Release, UpdateError>
  - `install_update(release: &Release)` → Result<(), UpdateError>
- **Release** struct: version, download URL, release notes, publish date
- **UpdateError** enum: NetworkError, ParseError, VersionMismatch, InstallError, etc.
- **UpdateStatus** enum: Checking, Available, Current, Error(UpdateError)

#### `src/state/update_state.rs`
- **UpdateState** struct:
  - `status: UpdateStatus` (current check status)
  - `available_version: Option<String>` (latest version if available)
  - `last_check_time: Option<SystemTime>` (when last check was performed)
  - `download_in_progress: bool`
  - `downloaded_binary_path: Option<PathBuf>` (temp location of downloaded binary)

### Configuration (`src/config.rs`)
- Add to **AppConfig**:
  - `check_updates_on_startup: bool` (default: true, user-configurable)
  - `last_check_timestamp: Option<u64>` (persisted to config.toml)

### State Management
- Update state is global (AppState level), not per-workspace
- Update check is triggered during app initialization
- Result is sent to main event loop via worker channel

### Worker Integration
- Add **UpdateCheckRequest** / **UpdateCheckResult** to existing worker message types
- Uses existing `crossbeam-channel` pattern for background communication
- Spawns optional background task on startup

---

## Implementation Details

### GitHub API Integration

**Endpoint:** `https://api.github.com/repos/tzero86/Zeta/releases/latest`

**Request:**
```
GET /repos/tzero86/Zeta/releases/latest HTTP/1.1
Host: api.github.com
User-Agent: Zeta/0.4.5
```

**Response (JSON):**
```json
{
  "tag_name": "v0.5.0",
  "name": "v0.5.0: Major improvements",
  "body": "## Features\n- ...\n## Bugfixes\n- ...",
  "prerelease": false,
  "published_at": "2026-04-29T12:00:00Z"
}
```

**Parsing:**
- Extract version from `tag_name` (strip 'v' prefix)
- Compare to current version (`env!("CARGO_PKG_VERSION")`)
- Return Release struct if newer

### Version Comparison

```rust
fn is_newer(current: &str, available: &str) -> bool {
    // semver comparison: e.g., "0.4.5" < "0.5.0"
    // Use basic comparison: split by dots, compare numerically
}
```

### HTTP Client

**Dependency:** Add `ureq` (lightweight, no async, fits sync architecture)
- No TLS verification bypass
- 5-second timeout for requests
- Follows GitHub redirects

### Binary Installation

**For `cargo install` installations:**
```bash
cargo install --git https://github.com/tzero86/Zeta
```

Implementation:
1. Download binary from GitHub Releases asset (e.g., `zeta-linux-x86_64.tar.gz` or `zeta-windows-x86_64.zip`)
2. Extract to temp directory
3. Replace current binary (located via `std::env::current_exe()`)
4. On Windows: handle in-use file (rename old binary, replace with new one)
5. Prompt user to restart

**Limitations:**
- Only works if Zeta was installed via `cargo install --git` to a known location
- Doesn't support custom installation paths (e.g., system package managers)
- Requires write permissions to binary location

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Network timeout | Log silently, don't show error to user (startup check mustn't block) |
| GitHub API error (non-200) | Log error, show optional retry in dialog |
| Failed version parse | Treat as check failed, log with details |
| Downloaded binary corrupted | Show error, suggest manual download from GitHub |
| Permission denied on binary replace | Show error with instructions (run with `sudo`, check permissions) |
| Already on latest version | Don't show notification, silently update state |

### State Persistence

- **last_check_timestamp** persisted to config.toml
- Survives app restart
- Used to implement future "don't check more than once per day" logic (if added later)

---

## UI/UX Specification

### Status Bar Indicator

**Location:** Bottom-right status bar, next to clock

**Design:**
- Pulsing red dot + "Update" text
- Animation: opacity fade 1.0 → 0.4 over ~1.5s cycle
- Only shown if update available (disappears once applied or dismissed permanently)

**Interaction:**
- Click to open update dialog
- Color: error red (default palette's error color, typically `#d32f2f` or theme-equivalent)

### Help Menu Entry

**Placement:** After "Documentation" entry, separated by divider

**States:**
1. **No update available:**
   ```
   Check for Updates
   ```
   (normal text, selectable)

2. **Update available:**
   ```
   🔴 Check for Updates
   → v0.5.0 available
   ```
   (red dot, version hint on second line)

3. **Checking:**
   ```
   ⏳ Check for Updates
   (checking...)
   ```

**Interaction:**
- Select to open update dialog or re-trigger check

### Update Dialog

**Title:** "Update Available" (or "Check for Updates" if checking)

**Content:**
```
Current Version: v0.4.5
Available Version: v0.5.0

[Release notes preview - first 200 chars]
...

[Download & Install]  [Later]
```

**States:**
1. **Checking:** Show spinner, disable buttons
2. **Available:** Show version info, enable Download button
3. **Downloading:** Show progress bar, disable Download button
4. **Ready to restart:** Show "Restart Now" button instead of Download
5. **Error:** Show error message, "Retry" button

### Dismissal

- Clicking "Later" closes dialog, but indicator persists
- Indicator remains visible until:
  - User restarts Zeta (check runs again)
  - User applies the update and restarts
  - New check finds the user is now up-to-date

---

## Data Flow Diagram

```
App startup
    ↓
Initialize state & UI (non-blocking)
    ↓
Spawn UpdateCheckRequest → worker thread
    ↓
[Background]              [Main thread]
Worker queries GitHub API → Receives UpdateCheckResult
    ↓                        ↓
Parse version            Merge result into UpdateState
    ↓                        ↓
Send result via channel  Next frame renders status bar hint
    ↓                        ↓
User clicks indicator → Open UpdateDialog
    ↓
User confirms download → Spawn UpdateInstallRequest
    ↓
[Background]              [Main thread]
Download + install binary → Receive result
    ↓                        ↓
Extract & validate       Show restart prompt
    ↓                        ↓
Replace current binary   User restarts
    ↓
Old binary replaced → New version runs
```

---

## Testing Strategy

### Unit Tests (`src/update.rs`)
- Version comparison logic (e.g., "0.4.5" < "0.5.0")
- JSON parsing with various API response formats
- Error scenarios (malformed JSON, missing fields)

### Integration Tests (`tests/`)
- Mock GitHub API responses
- Verify no crashes on network errors
- Test state transitions (checking → available → downloading → ready)

### Manual Testing
- [ ] Startup check completes without blocking (measure < 500ms)
- [ ] Status bar indicator appears when update available
- [ ] Help menu entry shows version correctly
- [ ] Clicking indicator opens dialog
- [ ] Download progress shows and completes
- [ ] Binary replacement works (test with manual installation)
- [ ] Restart applies new version
- [ ] Error scenarios (timeout, invalid JSON) fail gracefully

---

## Dependencies

**New crate:** `ureq` (~50KB uncompressed)
- Lightweight HTTP client
- No async runtime
- No TLS verification bypass (safe by default)

**No changes to existing dependencies**

---

## Limitations & Future Work

### Current Limitations
- Only works for `cargo install --git` installations
- No support for system package managers (apt, brew, etc.)
- Binary verification limited to version number (not cryptographic checksums)
- No pre-release version handling

### Future Enhancements
- Configurable check frequency (once per day, weekly, etc.)
- Checksum verification of downloaded binary
- Release notes display (full text, not just preview)
- Rollback to previous version
- Support for `cargo binstall` (faster downloads)
- Custom update server (for future off-GitHub distributions)

---

## Configuration Example

```toml
# Check for updates on startup (default: true)
check_updates_on_startup = true

# Last check timestamp (auto-managed, ISO 8601)
# last_check_timestamp = "2026-04-29T21:00:00Z"
```

No user configuration needed for first version — defaults are sensible.

---

## Rollback Plan

If the feature causes issues:
1. Set `check_updates_on_startup = false` in config
2. Comment out or disable the startup check in `app.rs`
3. Feature can be disabled without removing code (clean removal if needed)

---

## Acceptance Criteria

- ✅ Startup check is non-blocking (< 500ms perceived latency)
- ✅ Status bar shows pulsing indicator when update available
- ✅ Help menu entry reflects current state (checking, available, etc.)
- ✅ User can download and install update in 2-3 clicks
- ✅ Requires explicit user confirmation (no auto-install)
- ✅ Network errors fail gracefully without crashing
- ✅ Binary replacement works for typical `cargo install` paths
- ✅ All tests pass (unit + integration + manual smoke test)
- ✅ Zero new security warnings or clippy issues

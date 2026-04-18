"""
Patch src/pane.rs:
  1. Add `use std::time::SystemTime;` to imports
  2. Add ScanCache struct after the PaneMode block
  3. Add `scan_cache: Option<ScanCache>` field to PaneState
  4. Initialise scan_cache: None in PaneState::empty()
"""
import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\pane.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

# ── 1. Add SystemTime import ────────────────────────────────────────────────
old_import = "use std::collections::BTreeSet;\nuse std::path::PathBuf;"
new_import = "use std::collections::BTreeSet;\nuse std::path::PathBuf;\nuse std::time::SystemTime;"
if old_import not in text:
    print("ERROR: import block not found"); sys.exit(1)
text = text.replace(old_import, new_import, 1)

# ── 2. Add ScanCache struct before PaneState ────────────────────────────────
#    Insert right before `#[derive(Clone, Debug)]\npub struct PaneState {`
scan_cache_struct = (
    "/// Cached result of the last successful directory scan for one pane.\n"
    "///\n"
    "/// Held in `PaneState::scan_cache`. The cache is considered fresh when\n"
    "/// the directory's modification time has not changed since the scan.\n"
    "#[derive(Clone, Debug)]\n"
    "pub struct ScanCache {\n"
    "    /// The directory that was scanned.\n"
    "    pub path: PathBuf,\n"
    "    /// Modification time of `path` at scan time.\n"
    "    pub dir_mtime: SystemTime,\n"
    "    /// The raw entries returned by the scan (before \"..\" is prepended).\n"
    "    pub entries: Vec<crate::fs::EntryInfo>,\n"
    "}\n"
    "\n"
    "impl ScanCache {\n"
    "    /// Return `true` when the cached entries are still valid for `path`.\n"
    "    ///\n"
    "    /// Validity is defined as: the path matches AND the OS-reported\n"
    "    /// modification time of the directory equals the recorded mtime.\n"
    "    pub fn is_fresh(&self, path: &std::path::Path) -> bool {\n"
    "        if self.path != path {\n"
    "            return false;\n"
    "        }\n"
    "        std::fs::metadata(path)\n"
    "            .and_then(|m| m.modified())\n"
    "            .map(|mtime| mtime == self.dir_mtime)\n"
    "            .unwrap_or(false)\n"
    "    }\n"
    "}\n"
    "\n"
)

old_pane_state_header = "#[derive(Clone, Debug)]\npub struct PaneState {"
if old_pane_state_header not in text:
    print("ERROR: PaneState header not found"); sys.exit(1)
text = text.replace(old_pane_state_header, scan_cache_struct + old_pane_state_header, 1)

# ── 3. Add scan_cache field to PaneState ────────────────────────────────────
old_field = "    pub mode: PaneMode, // New: real fs or archive mode\n}"
new_field = (
    "    pub mode: PaneMode, // New: real fs or archive mode\n"
    "    /// Cached result of the last completed directory scan.\n"
    "    pub scan_cache: Option<ScanCache>,\n"
    "}"
)
if old_field not in text:
    print("ERROR: mode field not found"); sys.exit(1)
text = text.replace(old_field, new_field, 1)

# ── 4. Initialise scan_cache in empty() ─────────────────────────────────────
old_init = (
    "            history_back: Vec::new(),\n"
    "            history_forward: Vec::new(),\n"
    "            filtered_indices: RefCell::new(Vec::new()),"
)
new_init = (
    "            history_back: Vec::new(),\n"
    "            history_forward: Vec::new(),\n"
    "            scan_cache: None,\n"
    "            filtered_indices: RefCell::new(Vec::new()),"
)
if old_init not in text:
    print("ERROR: init block not found"); sys.exit(1)
text = text.replace(old_init, new_init, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: pane.rs patched (ScanCache)")

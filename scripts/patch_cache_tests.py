"""
Add three ScanCache tests to src/state/pane_set.rs:
  1. refresh_with_fresh_cache_skips_scan   — cache fresh → no ScanPane emitted
  2. refresh_with_stale_mtime_queues_scan  — wrong mtime → ScanPane emitted
  3. refresh_with_no_cache_queues_scan     — no cache at all → ScanPane emitted
"""
import sys, os, tempfile, time

path = r"C:\Users\Zero\Documents\coding\Zeta\src\state\pane_set.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

# Append before the closing `}` of mod tests
old_end = "    #[test]\n    fn inactive_pane_returns_opposite_of_focus() {"
new_test = (
    "    #[test]\n"
    "    fn refresh_with_fresh_cache_skips_scan() {\n"
    "        // Create a real temp directory so mtime queries succeed.\n"
    "        let dir = tempfile::tempdir().expect(\"temp dir\");\n"
    "        let path = dir.path().to_path_buf();\n"
    "        let mtime = std::fs::metadata(&path)\n"
    "            .expect(\"metadata\")\n"
    "            .modified()\n"
    "            .expect(\"mtime\");\n"
    "        let mut s = PaneSetState::new(\n"
    "            PaneState::empty(\"Left\", path.clone()),\n"
    "            PaneState::empty(\"Right\", std::env::temp_dir()),\n"
    "        );\n"
    "        s.left.scan_cache = Some(crate::pane::ScanCache {\n"
    "            path: path.clone(),\n"
    "            dir_mtime: mtime,\n"
    "            entries: vec![],\n"
    "        });\n"
    "        let cmds = s.apply(&Action::Refresh).unwrap();\n"
    "        let has_scan = cmds.iter().any(|c| matches!(c, Command::ScanPane { .. }));\n"
    "        assert!(!has_scan, \"fresh cache should suppress ScanPane\");\n"
    "    }\n"
    "\n"
    "    #[test]\n"
    "    fn refresh_with_stale_mtime_queues_scan() {\n"
    "        let dir = tempfile::tempdir().expect(\"temp dir\");\n"
    "        let path = dir.path().to_path_buf();\n"
    "        // Use UNIX_EPOCH as a deliberately wrong mtime.\n"
    "        let stale_mtime = std::time::UNIX_EPOCH;\n"
    "        let mut s = PaneSetState::new(\n"
    "            PaneState::empty(\"Left\", path.clone()),\n"
    "            PaneState::empty(\"Right\", std::env::temp_dir()),\n"
    "        );\n"
    "        s.left.scan_cache = Some(crate::pane::ScanCache {\n"
    "            path: path.clone(),\n"
    "            dir_mtime: stale_mtime,\n"
    "            entries: vec![],\n"
    "        });\n"
    "        let cmds = s.apply(&Action::Refresh).unwrap();\n"
    "        let has_scan = cmds.iter().any(|c| matches!(c, Command::ScanPane { .. }));\n"
    "        assert!(has_scan, \"stale mtime should trigger ScanPane\");\n"
    "    }\n"
    "\n"
    "    #[test]\n"
    "    fn refresh_with_no_cache_queues_scan() {\n"
    "        let cwd = std::env::temp_dir();\n"
    "        let mut s = PaneSetState::new(\n"
    "            PaneState::empty(\"Left\", cwd.clone()),\n"
    "            PaneState::empty(\"Right\", cwd),\n"
    "        );\n"
    "        assert!(s.left.scan_cache.is_none());\n"
    "        let cmds = s.apply(&Action::Refresh).unwrap();\n"
    "        let has_scan = cmds.iter().any(|c| matches!(c, Command::ScanPane { .. }));\n"
    "        assert!(has_scan, \"missing cache should trigger ScanPane\");\n"
    "    }\n"
    "\n"
    "    #[test]\n"
    "    fn inactive_pane_returns_opposite_of_focus() {"
)

if old_end not in text:
    print("ERROR: anchor not found")
    sys.exit(1)

text = text.replace(old_end, new_test, 1)

# Add tempfile import inside mod tests
old_use = "    use super::*;\n    use crate::pane::PaneState;\n"
new_use = "    use super::*;\n    use crate::pane::PaneState;\n    use crate::action::Command;\n"
if old_use in text:
    text = text.replace(old_use, new_use, 1)
else:
    print("WARNING: use block not found, skipping Command import")

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: pane_set.rs tests added")

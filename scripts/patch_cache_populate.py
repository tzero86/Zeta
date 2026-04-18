"""
Patch src/state/mod.rs inside the DirectoryScanned handler:
  Populate scan_cache on a successful local scan (not archive, not remote).
  Insert right after set_entries / refresh_filter and before the pending_reveal block.
"""
import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\state\mod.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = (
    "                self.panes.pane_mut(pane).set_entries(all_entries);\n"
    "                self.panes.pane_mut(pane).refresh_filter();\n"
    "                if let Some((pending_pane, pending_path)) = self.pending_reveal.clone() {"
)

new = (
    "                self.panes.pane_mut(pane).set_entries(all_entries.clone());\n"
    "                self.panes.pane_mut(pane).refresh_filter();\n"
    "                // Update the scan cache for local, non-archive panes.\n"
    "                if !self.panes.pane(pane).in_archive() && !self.panes.pane(pane).in_remote() {\n"
    "                    let dir_mtime = std::fs::metadata(&path)\n"
    "                        .and_then(|m| m.modified())\n"
    "                        .ok();\n"
    "                    if let Some(dir_mtime) = dir_mtime {\n"
    "                        // Strip the \"..\" sentinel before caching — it is re-added on restore.\n"
    "                        let cache_entries: Vec<crate::fs::EntryInfo> = all_entries\n"
    "                            .iter()\n"
    "                            .filter(|e| e.name != \"..\")\n"
    "                            .cloned()\n"
    "                            .collect();\n"
    "                        self.panes.pane_mut(pane).scan_cache =\n"
    "                            Some(crate::pane::ScanCache {\n"
    "                                path: path.clone(),\n"
    "                                dir_mtime,\n"
    "                                entries: cache_entries,\n"
    "                            });\n"
    "                    }\n"
    "                }\n"
    "                if let Some((pending_pane, pending_path)) = self.pending_reveal.clone() {"
)

if old not in text:
    print("ERROR: pattern not found")
    idx = text.find("set_entries(all_entries)")
    if idx >= 0:
        print(repr(text[max(0,idx-50):idx+400]))
    else:
        print("set_entries(all_entries) not found either")
    sys.exit(1)

text = text.replace(old, new, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: state/mod.rs patched (populate scan_cache on DirectoryScanned)")

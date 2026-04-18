"""
Patch src/state/pane_set.rs:
  - Action::Refresh: check scan_cache.is_fresh(); skip ScanPane if cache is valid.
    Always force a scan when the pane is in archive or remote mode.
"""
import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\state\pane_set.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = (
    "            Action::Refresh => {\n"
    "                let pane = self.focused_pane_id();\n"
    "                let path = self.active_pane().cwd.clone();\n"
    "                commands.push(Command::ScanPane { pane, path });\n"
    "            }\n"
)

new = (
    "            Action::Refresh => {\n"
    "                let pane = self.focused_pane_id();\n"
    "                let active = self.active_pane();\n"
    "                let path = active.cwd.clone();\n"
    "                // Skip the background scan when:\n"
    "                //   (a) the pane is on a real local filesystem, AND\n"
    "                //   (b) we have a cached scan result, AND\n"
    "                //   (c) the directory's mtime has not changed since the scan.\n"
    "                // In all other cases we always re-scan.\n"
    "                let cache_hit = !active.in_archive()\n"
    "                    && !active.in_remote()\n"
    "                    && active\n"
    "                        .scan_cache\n"
    "                        .as_ref()\n"
    "                        .map(|c| c.is_fresh(&path))\n"
    "                        .unwrap_or(false);\n"
    "                if !cache_hit {\n"
    "                    commands.push(Command::ScanPane { pane, path });\n"
    "                }\n"
    "            }\n"
)

if old not in text:
    print("ERROR: pattern not found")
    idx = text.find("Action::Refresh")
    if idx >= 0:
        print(repr(text[max(0,idx-50):idx+400]))
    sys.exit(1)

text = text.replace(old, new, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: pane_set.rs patched (cache-aware Refresh)")

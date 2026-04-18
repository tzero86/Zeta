import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\ui\pane.rs"
with open(path, "rb") as f:
    raw = f.read()

crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = (
    "    let name = display_name.unwrap_or_else(|| match entry.kind {\n"
    "        EntryKind::Directory => format!(\"{}/\", entry.name),\n"
    "        _ => entry.name.clone(),\n"
    "    });"
)

arrow = "\u2192"
new = (
    "    let name = display_name.unwrap_or_else(|| match entry.kind {\n"
    "        EntryKind::Directory => format!(\"{}/\", entry.name),\n"
    "        EntryKind::Symlink => {\n"
    "            if let Some(ref target) = entry.link_target {\n"
    "                let target_str = target.to_string_lossy();\n"
    f"                format!(\"{{}} {arrow} {{}}\", entry.name, target_str)\n"
    "            } else {\n"
    "                entry.name.clone()\n"
    "            }\n"
    "        }\n"
    "        _ => entry.name.clone(),\n"
    "    });"
)

if old not in text:
    print("ERROR: pattern not found in", path)
    sys.exit(1)

text = text.replace(old, new, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")

with open(path, "wb") as f:
    f.write(out)

print("OK: ui/pane.rs patched")

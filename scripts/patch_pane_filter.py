import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\pane.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

# Replace the substring filter line with the glob matcher call
old = (
    "        if self.filter_active && !self.filter_query.is_empty() {\n"
    "            let query = self.filter_query.to_lowercase();\n"
    "            // \"..\"\u00a0is always visible even during filtering.\n"
    "            rest_indices.retain(|&idx| self.entries[idx].name.to_lowercase().contains(&query));\n"
    "        }"
)
# Try without non-breaking space
old2 = (
    "        if self.filter_active && !self.filter_query.is_empty() {\n"
    "            let query = self.filter_query.to_lowercase();\n"
    "            // \"..\" is always visible even during filtering.\n"
    "            rest_indices.retain(|&idx| self.entries[idx].name.to_lowercase().contains(&query));\n"
    "        }"
)

new = (
    "        if self.filter_active && !self.filter_query.is_empty() {\n"
    "            // \"..\" is always visible even during filtering.\n"
    "            rest_indices.retain(|&idx| {\n"
    "                crate::utils::glob_match::matches(\n"
    "                    &self.filter_query,\n"
    "                    &self.entries[idx].name,\n"
    "                )\n"
    "            });\n"
    "        }"
)

if old in text:
    text = text.replace(old, new, 1)
    print("OK (variant 1)")
elif old2 in text:
    text = text.replace(old2, new, 1)
    print("OK (variant 2)")
else:
    # Search for the block and show context
    idx = text.find("rest_indices.retain")
    if idx >= 0:
        print("Found retain at", idx)
        print(repr(text[max(0,idx-200):idx+300]))
    else:
        print("ERROR: retain not found at all")
    sys.exit(1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: pane.rs patched")

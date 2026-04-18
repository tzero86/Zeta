import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\state\mod.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

# Fix both PaneState struct literals in test helpers — they're identical shape
# but appear at different locations. Add scan_cache: None to each.
old_tail = (
    "            cache_sort_mode: std::cell::Cell::new(SortMode::Name),\n"
    "            cache_filter_active: std::cell::Cell::new(false),\n"
    "            cache_filter_query: std::cell::RefCell::new(String::new()),\n"
    "            mode: crate::pane::PaneMode::Real,\n"
    "            mark_anchor: None,\n"
    "            details_view: false,\n"
    "            rename_state: None,\n"
    "        }\n"
    "    }\n"
)

new_tail = (
    "            cache_sort_mode: std::cell::Cell::new(SortMode::Name),\n"
    "            cache_filter_active: std::cell::Cell::new(false),\n"
    "            cache_filter_query: std::cell::RefCell::new(String::new()),\n"
    "            mode: crate::pane::PaneMode::Real,\n"
    "            mark_anchor: None,\n"
    "            details_view: false,\n"
    "            rename_state: None,\n"
    "            scan_cache: None,\n"
    "        }\n"
    "    }\n"
)

count = text.count(old_tail)
if count == 0:
    print("ERROR: pattern not found")
    sys.exit(1)
print(f"Found {count} occurrence(s), replacing all")
text = text.replace(old_tail, new_tail)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK")

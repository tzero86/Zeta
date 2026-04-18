import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\action.rs"
with open(path, "rb") as f:
    raw = f.read()

crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = (
    "            KeyCode::Enter | KeyCode::Right => Some(Self::EnterSelection),\n"
    "            // Char('l') without Ctrl is the vim right/enter binding.\n"
    "            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::NONE => {\n"
    "                Some(Self::EnterSelection)\n"
    "            }\n"
)

new = (
    "            KeyCode::Enter | KeyCode::Right => Some(Self::EnterSelection),\n"
    "            // Char('l') without Ctrl is the vim right/enter binding.\n"
    "            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::NONE => {\n"
    "                Some(Self::EnterSelection)\n"
    "            }\n"
    "            // Alt+Enter follows a symlink into its target directory / file.\n"
    "            KeyCode::Enter if key_event.modifiers == KeyModifiers::ALT => {\n"
    "                Some(Self::FollowSymlink)\n"
    "            }\n"
    "            // Alt+i shows the symlink target path in the status bar.\n"
    "            KeyCode::Char('i') if key_event.modifiers == KeyModifiers::ALT => {\n"
    "                Some(Self::ShowSymlinkTarget)\n"
    "            }\n"
)

if old not in text:
    print("ERROR: pattern not found")
    sys.exit(1)

text = text.replace(old, new, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")

with open(path, "wb") as f:
    f.write(out)

print("OK: key bindings added")

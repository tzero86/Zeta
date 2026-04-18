import sys

# Fix 1: wrap the bare `return vec![...]` in Ok() in state/mod.rs
path = r"C:\Users\Zero\Documents\coding\Zeta\src\state\mod.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = "                                return vec![Command::DispatchAction(Action::Refresh)];"
new = "                                return Ok(vec![Command::DispatchAction(Action::Refresh)]);"

if old not in text:
    print("ERROR: state pattern not found")
    sys.exit(1)
text = text.replace(old, new, 1)

old2 = (
    "                            } else if target.is_file() {\n"
    "                                return vec![Command::DispatchAction(\n"
    "                                    Action::OpenSelectedInEditor,\n"
    "                                )];"
)
new2 = (
    "                            } else if target.is_file() {\n"
    "                                return Ok(vec![Command::DispatchAction(\n"
    "                                    Action::OpenSelectedInEditor,\n"
    "                                )]);"
)
if old2 not in text:
    print("ERROR: state pattern2 not found")
    sys.exit(1)
text = text.replace(old2, new2, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: state/mod.rs fixed")

# Fix 2: change Alt+Enter to Alt+l in action.rs (Enter guard unreachable after bare Enter)
path2 = r"C:\Users\Zero\Documents\coding\Zeta\src\action.rs"
with open(path2, "rb") as f:
    raw2 = f.read()
crlf2 = b"\r\n" in raw2
text2 = raw2.replace(b"\r\n", b"\n").decode("utf-8")

old3 = (
    "            // Alt+Enter follows a symlink into its target directory / file.\n"
    "            KeyCode::Enter if key_event.modifiers == KeyModifiers::ALT => {\n"
    "                Some(Self::FollowSymlink)\n"
    "            }\n"
)
new3 = (
    "            // Alt+l follows a symlink into its target directory / file.\n"
    "            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::ALT => {\n"
    "                Some(Self::FollowSymlink)\n"
    "            }\n"
)
if old3 not in text2:
    print("ERROR: action pattern not found")
    sys.exit(1)
text2 = text2.replace(old3, new3, 1)

out2 = text2.encode("utf-8")
if crlf2:
    out2 = out2.replace(b"\n", b"\r\n")
with open(path2, "wb") as f:
    f.write(out2)
print("OK: action.rs fixed (Alt+l binding)")

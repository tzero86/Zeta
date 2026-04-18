import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\state\mod.rs"
with open(path, "rb") as f:
    raw = f.read()

crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

# Fix the FollowSymlink handler - push_history() reads self.cwd internally, no arg needed
old = (
    "            Action::FollowSymlink => {\n"
    "                if let Some(entry) = self.panes.active_pane().selected_entry().cloned() {\n"
    "                    if entry.kind == EntryKind::Symlink {\n"
    "                        if let Some(ref target) = entry.link_target {\n"
    "                            if target.is_dir() {\n"
    "                                let pane = self.panes.active_pane_mut();\n"
    "                                let old_cwd = pane.cwd.clone();\n"
    "                                pane.push_history(old_cwd);\n"
    "                                pane.cwd = target.clone();\n"
    "                                return vec![Command::DispatchAction(Action::Refresh)];\n"
    "                            } else if target.is_file() {\n"
    "                                return vec![Command::DispatchAction(\n"
    "                                    Action::OpenSelectedInEditor,\n"
    "                                )];\n"
    "                            } else {\n"
    "                                self.status_message = format!(\"target does not exist: {}\", target.display());\n"
    "                            }\n"
    "                        }\n"
    "                    }\n"
    "                }\n"
    "            }\n"
)

new = (
    "            Action::FollowSymlink => {\n"
    "                if let Some(entry) = self.panes.active_pane().selected_entry().cloned() {\n"
    "                    if entry.kind == EntryKind::Symlink {\n"
    "                        if let Some(target) = entry.link_target {\n"
    "                            if target.is_dir() {\n"
    "                                let pane = self.panes.active_pane_mut();\n"
    "                                pane.push_history();\n"
    "                                pane.cwd = target;\n"
    "                                return vec![Command::DispatchAction(Action::Refresh)];\n"
    "                            } else if target.is_file() {\n"
    "                                return vec![Command::DispatchAction(\n"
    "                                    Action::OpenSelectedInEditor,\n"
    "                                )];\n"
    "                            } else {\n"
    "                                self.status_message =\n"
    "                                    format!(\"target does not exist: {}\", target.display());\n"
    "                            }\n"
    "                        }\n"
    "                    }\n"
    "                }\n"
    "            }\n"
)

if old not in text:
    print("ERROR: pattern not found")
    # show context around FollowSymlink
    idx = text.find("Action::FollowSymlink")
    print(repr(text[idx:idx+600]))
    sys.exit(1)

text = text.replace(old, new, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")

with open(path, "wb") as f:
    f.write(out)

print("OK: fixed push_history() call")

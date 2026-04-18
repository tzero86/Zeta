import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\action.rs"
with open(path, "rb") as f:
    raw = f.read()

crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = (
    "    /// User accepted an unknown SSH host key and wants to proceed.\n"
    "    SshTrustAccept,\n"
    "    /// User rejected an unknown SSH host key; cancel the connection.\n"
    "    SshTrustReject,\n"
    "}"
)

new = (
    "    /// User accepted an unknown SSH host key and wants to proceed.\n"
    "    SshTrustAccept,\n"
    "    /// User rejected an unknown SSH host key; cancel the connection.\n"
    "    SshTrustReject,\n"
    "    /// Navigate the active pane into the symlink's resolved target directory (or open the\n"
    "    /// target file). No-op if the focused entry is not a symlink or the target does not exist.\n"
    "    FollowSymlink,\n"
    "    /// Display the symlink's resolved target path in the status bar without navigating.\n"
    "    ShowSymlinkTarget,\n"
    "}"
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

print("OK")

import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\lib.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = "pub mod ui;\n\npub use app::App;"
new = "pub mod ui;\npub mod utils;\n\npub use app::App;"

if old not in text:
    print("ERROR: pattern not found")
    print(repr(text))
    sys.exit(1)
text = text.replace(old, new, 1)

out = text.encode("utf-8")
if crlf:
    out = out.replace(b"\n", b"\r\n")
with open(path, "wb") as f:
    f.write(out)
print("OK: lib.rs patched")

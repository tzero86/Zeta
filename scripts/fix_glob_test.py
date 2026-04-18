import sys

path = r"C:\Users\Zero\Documents\coding\Zeta\src\utils\glob_match.rs"
with open(path, "rb") as f:
    raw = f.read()
crlf = b"\r\n" in raw
text = raw.replace(b"\r\n", b"\n").decode("utf-8")

old = (
    "    #[test]\n"
    "    fn mixed_wildcards() {\n"
    "        assert!(matches(\"f*o?.rs\", \"foobar.rs\"));\n"
    "        assert!(!matches(\"f*o?.rs\", \"foobar.toml\"));\n"
    "    }\n"
)

new = (
    "    #[test]\n"
    "    fn mixed_wildcards() {\n"
    "        // f* matches 'foob', ? matches 'a', .rs matches .rs\n"
    "        assert!(matches(\"f*?.rs\", \"foobar.rs\"));\n"
    "        assert!(!matches(\"f*?.rs\", \"foobar.toml\"));\n"
    "        // f matches f, * matches 'oob', a matches a, r.rs matches r.rs\n"
    "        assert!(matches(\"f*ar.rs\", \"foobar.rs\"));\n"
    "        // pattern with both * and ?: f, anything, o, single-char, .rs\n"
    "        assert!(matches(\"f*o?.rs\", \"fXoY.rs\"));\n"
    "        assert!(!matches(\"f*o?.rs\", \"fXoYZ.rs\"));\n"
    "    }\n"
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

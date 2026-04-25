/// Case-insensitive substring split that is safe for all Unicode.
///
/// Searches `s` for the first occurrence of `needle_lower` (which must already
/// be lowercased) using char-by-char comparison. Returns `(before, matched, after)`
/// slices of the original `s`, preserving the original casing of the match.
pub fn split_at_match<'a>(s: &'a str, needle_lower: &str) -> Option<(&'a str, &'a str, &'a str)> {
    if needle_lower.is_empty() {
        return None;
    }
    let needle_chars: Vec<char> = needle_lower.chars().collect();
    let needle_len = needle_chars.len();
    let char_indices: Vec<(usize, char)> = s.char_indices().collect();
    for start_ci in 0..char_indices.len() {
        if start_ci + needle_len > char_indices.len() {
            break;
        }
        let matches = char_indices[start_ci..start_ci + needle_len]
            .iter()
            .zip(needle_chars.iter())
            .all(|((_, c), n)| c.to_lowercase().eq(std::iter::once(*n)));
        if matches {
            let start_byte = char_indices[start_ci].0;
            let end_byte = char_indices
                .get(start_ci + needle_len)
                .map(|(b, _)| *b)
                .unwrap_or(s.len());
            return Some((&s[..start_byte], &s[start_byte..end_byte], &s[end_byte..]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::split_at_match;

    #[test]
    fn ascii_match() {
        let result = split_at_match("hello world", "world");
        assert_eq!(result, Some(("hello ", "world", "")));
    }

    #[test]
    fn case_insensitive_ascii() {
        let result = split_at_match("Hello World", "world");
        assert_eq!(result, Some(("Hello ", "World", "")));
    }

    #[test]
    fn no_match() {
        assert_eq!(split_at_match("hello", "xyz"), None);
    }

    #[test]
    fn empty_needle_returns_none() {
        assert_eq!(split_at_match("hello", ""), None);
    }

    #[test]
    fn unicode_non_expanding() {
        // é (1 char, 2 bytes UTF-8)
        let result = split_at_match("café", "é");
        assert_eq!(result, Some(("caf", "é", "")));
    }

    #[test]
    fn unicode_expanding_lowercase() {
        // İ (U+0130, 2 bytes) — lowercases to multi-byte sequence in some contexts
        // This must not panic
        let result = split_at_match("İstanbul", "i");
        // Should find the first char-level match, not panic
        let _ = result; // just ensure no panic
    }
}

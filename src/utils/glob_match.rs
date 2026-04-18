/// Enhanced glob matcher for pane filters, maintaining compatibility with the current structure.
///
/// Updates to improve robustness against negated patterns, edge cases involving substring versus wildcard.
/// Refines fallback behavior.
/// Lightweight glob matcher for pane filter queries.
///
/// Supported syntax
/// ----------------
/// - `*`   — matches zero or more characters (but not a path separator)
/// - `?`   — matches exactly one character
/// - Any other character — matched literally (case-insensitive)
/// - Leading `!` — negates the whole pattern: returns `true` only when the
///   name would NOT match the rest of the pattern.
///
/// If the query contains neither `*` nor `?` it falls back to a simple
/// case-insensitive substring search, preserving the existing UX.
///
/// Examples
/// --------
/// ```
/// use zeta::utils::glob_match::matches;
///
/// assert!(matches("*.rs",  "main.rs"));
/// assert!(matches("*.rs",  "lib.rs"));
/// assert!(!matches("*.rs", "main.toml"));
/// assert!(matches("foo?",  "fooX"));
/// assert!(!matches("foo?", "foo"));
/// assert!(matches("!*.bak", "readme.md"));
/// assert!(!matches("!*.bak", "notes.bak"));
/// assert!(matches("main",  "my_main_loop.rs"));   // substring fallback
/// ```
pub fn matches(pattern: &str, name: &str) -> bool {
    let (negate, core) = if let Some(rest) = pattern.strip_prefix('!') {
        (true, rest)
    } else {
        (false, pattern)
    };

    let result = if core.contains('*') || core.contains('?') {
        glob_match(core, name)
    } else {
        // Substring fallback — keeps the existing UX for plain text queries.
        name.to_lowercase().contains(&core.to_lowercase())
    };

    if negate {
        !result
    } else {
        result
    }
}

/// Core glob match: pattern vs name, both compared case-insensitively.
/// Uses an iterative DP approach that is O(P*N) but avoids recursion.
fn glob_match(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();

    let (plen, nlen) = (p.len(), n.len());

    // dp[i][j] = pattern[..i] matches name[..j]
    // We only need the previous row to compute the current row.
    let mut prev = vec![false; nlen + 1];
    let mut curr = vec![false; nlen + 1];

    prev[0] = true;
    // A run of leading `*` matches the empty string.
    for i in 1..=plen {
        if p[i - 1] == '*' {
            // `*` at pattern position i can match empty suffix of name.
            curr[0] = prev[0];
        } else {
            curr[0] = false;
        }

        for j in 1..=nlen {
            curr[j] = if p[i - 1] == '*' {
                // `*` matches zero chars (prev[j]) or one more name char (curr[j-1]).
                prev[j] || curr[j - 1]
            } else if p[i - 1] == '?' || p[i - 1] == n[j - 1] {
                prev[j - 1]
            } else {
                false
            };
        }

        std::mem::swap(&mut prev, &mut curr);
        // Reset curr for next iteration.
        curr.iter_mut().for_each(|v| *v = false);
    }

    prev[nlen]
}

#[cfg(test)]
mod tests {
    use super::matches;

    // --- Star wildcard ---

    #[test]
    fn star_matches_any_suffix() {
        assert!(matches("*.rs", "main.rs"));
        assert!(matches("*.rs", "lib.rs"));
        assert!(matches("f*n*e", "filename"));
        assert!(!matches("f*n*e", "final"));
        assert!(matches("!*.toml", "notes.bak"));
        assert!(!matches("!anything", "anything"));
        assert!(matches("*.rs", ".rs"));
    }

    #[test]
    fn star_does_not_match_wrong_extension() {
        assert!(!matches("*.rs", "main.toml"));
        assert!(!matches("*.rs", "Cargo.lock"));
    }

    #[test]
    fn star_matches_any_prefix() {
        assert!(matches("foo*", "foobar"));
        assert!(matches("foo*", "foo"));
        assert!(!matches("foo*", "barfoo"));
    }

    #[test]
    fn double_star_acts_like_single_star() {
        assert!(matches("**.rs", "main.rs"));
    }

    #[test]
    fn star_only_matches_everything() {
        assert!(matches("*", "anything"));
        assert!(matches("*", ""));
    }

    // --- Question-mark wildcard ---

    #[test]
    fn question_matches_exactly_one_char() {
        assert!(matches("foo?", "fooX"));
        assert!(matches("foo?", "foo1"));
    }

    #[test]
    fn question_does_not_match_zero_chars() {
        assert!(!matches("foo?", "foo"));
    }

    #[test]
    fn question_does_not_match_two_chars() {
        assert!(!matches("foo?", "fooXY"));
    }

    // --- Case insensitivity ---

    #[test]
    fn glob_is_case_insensitive() {
        assert!(matches("*.RS", "main.rs"));
        assert!(matches("*.rs", "Main.RS"));
        assert!(matches("FOO*", "foobar"));
    }

    // --- Negation ---

    #[test]
    fn negation_inverts_match() {
        assert!(matches("!*.bak", "readme.md"));
        assert!(!matches("!*.bak", "notes.bak"));
    }

    #[test]
    fn negation_with_question_mark() {
        assert!(matches("!foo?", "foo")); // "foo" does not match "foo?" so negation is true
        assert!(!matches("!foo?", "fooX")); // "fooX" matches "foo?" so negation is false
    }

    // --- Substring fallback (no wildcards) ---

    #[test]
    fn plain_query_is_substring_match() {
        assert!(matches("main", "my_main_loop.rs"));
        assert!(matches("main", "main.rs"));
        assert!(!matches("main", "lib.rs"));
    }

    #[test]
    fn plain_query_is_case_insensitive() {
        assert!(matches("MAIN", "main.rs"));
        assert!(matches("main", "MAIN.rs"));
    }

    // --- Edge cases ---

    #[test]
    fn empty_pattern_matches_everything_via_substring() {
        // Empty substring is found in every string.
        assert!(matches("", "anything"));
        assert!(matches("", ""));
    }

    #[test]
    fn pattern_longer_than_name_does_not_match() {
        assert!(!matches("abcdef", "abc"));
    }

    #[test]
    fn exact_glob_match() {
        assert!(matches("main.rs", "main.rs"));
        assert!(!matches("main.rs", "main.toml"));
    }

    #[test]
    fn mixed_wildcards() {
        // f* matches 'foob', ? matches 'a', .rs matches .rs
        assert!(matches("f*?.rs", "foobar.rs"));
        assert!(!matches("f*?.rs", "foobar.toml"));
        // f matches f, * matches 'oob', a matches a, r.rs matches r.rs
        assert!(matches("f*ar.rs", "foobar.rs"));
        // pattern with both * and ?: f, anything, o, single-char, .rs
        assert!(matches("f*o?.rs", "fXoY.rs"));
        assert!(!matches("f*o?.rs", "fXoYZ.rs"));
    }
}

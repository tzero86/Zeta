use serde_json;
use std::cmp::Ordering;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Release {
    pub version: String,
    pub tag_name: String,
    pub body: String,
    pub prerelease: bool,
    pub published_at: String,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Checking,
    Available(Release),
    Current,
    Error(String),
}

#[derive(Debug, Error)]
pub enum UpdateError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Failed to parse JSON response: {0}")]
    JsonError(String),
    #[error("Version mismatch: current {current}, available {available}")]
    VersionMismatch { current: String, available: String },
    #[error("Failed to download binary: {0}")]
    DownloadError(String),
    #[error("Failed to install update: {0}")]
    InstallError(String),
}

/// Compare semantic versions: returns true if `available > current`.
/// Splits by dots, compares numeric parts left-to-right.
fn is_newer_version(current: &str, available: &str) -> bool {
    let current_parts: Vec<&str> = current.split('.').collect();
    let available_parts: Vec<&str> = available.split('.').collect();

    for i in 0..std::cmp::max(current_parts.len(), available_parts.len()) {
        let curr_num = current_parts
            .get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let avail_num = available_parts
            .get(i)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        match avail_num.cmp(&curr_num) {
            Ordering::Greater => return true,
            Ordering::Less => return false,
            Ordering::Equal => continue,
        }
    }
    false
}

/// Parse version from GitHub tag (e.g., "v0.5.0" -> "0.5.0", "0.5.0" -> "0.5.0").
/// Enforces strict semantic version shape: MAJOR.MINOR[.PATCH[...]]
/// Rejects malformed tags like "v1..0", "v.", "release-2026.04", or "foo1".
fn parse_version_tag(tag: &str) -> Option<String> {
    let v = tag.trim_start_matches('v');

    // Must match pattern: digits, optionally followed by (dot + digits) repeated
    // Valid: "0.5.0", "1", "1.0", "1.0.0.1"
    // Invalid: "1..0", ".", ".1", "1.", "foo1", "1a.0"
    if v.is_empty() {
        return None;
    }

    // Split by dots and validate each segment is non-empty and all digits
    v.split('.')
        .all(|segment| !segment.is_empty() && segment.chars().all(|c| c.is_numeric()))
        .then(|| v.to_string())
}

pub struct UpdateChecker;

impl UpdateChecker {
    /// Query GitHub API for the latest release of Zeta.
    /// Returns Release if found and newer, None if current, error otherwise.
    pub fn check_latest_release(current_version: &str) -> Result<Option<Release>, UpdateError> {
        let url = "https://api.github.com/repos/tzero86/Zeta/releases/latest";

        let mut resp = ureq::get(url)
            .header("User-Agent", &format!("Zeta/{}", current_version))
            .config()
            .timeout_global(Some(std::time::Duration::from_secs(5)))
            .build()
            .call()
            .map_err(|e: ureq::Error| UpdateError::NetworkError(e.to_string()))?;

        let body = resp
            .body_mut()
            .read_to_string()
            .map_err(|e: ureq::Error| UpdateError::NetworkError(e.to_string()))?;

        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| UpdateError::JsonError(format!("{}", e)))?;

        let tag_name = json["tag_name"]
            .as_str()
            .ok_or_else(|| UpdateError::JsonError("Missing tag_name".to_string()))?;

        let version = parse_version_tag(tag_name)
            .ok_or_else(|| UpdateError::JsonError("Invalid version format".to_string()))?;

        if !is_newer_version(current_version, &version) {
            return Ok(None); // Already on latest
        }

        let body = json["body"].as_str().unwrap_or("").to_string();
        let prerelease = json["prerelease"].as_bool().unwrap_or(false);
        let published_at = json["published_at"].as_str().unwrap_or("").to_string();

        // Skip pre-release versions to avoid offering unstable builds to stable users.
        if prerelease {
            return Ok(None);
        }

        Ok(Some(Release {
            version,
            tag_name: tag_name.to_string(),
            body,
            prerelease,
            published_at,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_version() {
        assert!(is_newer_version("0.4.5", "0.5.0"));
        assert!(is_newer_version("0.4.5", "0.4.6"));
        assert!(is_newer_version("0.4.5", "1.0.0"));
        assert!(!is_newer_version("0.5.0", "0.4.5"));
        assert!(!is_newer_version("0.5.0", "0.5.0"));
    }

    #[test]
    fn test_parse_version_tag() {
        // Valid semantic version tags
        assert_eq!(parse_version_tag("v0.5.0"), Some("0.5.0".to_string()));
        assert_eq!(parse_version_tag("0.5.0"), Some("0.5.0".to_string()));
        assert_eq!(parse_version_tag("1"), Some("1".to_string())); // single segment
        assert_eq!(parse_version_tag("v1.0"), Some("1.0".to_string())); // two segments
        assert_eq!(parse_version_tag("1.2.3.4"), Some("1.2.3.4".to_string())); // four segments

        // Reject malformed tags with empty segments or invalid characters
        assert_eq!(parse_version_tag("v1..0"), None); // double dots (empty segment)
        assert_eq!(parse_version_tag("v.1.0"), None); // starts with dot
        assert_eq!(parse_version_tag("v1.0."), None); // ends with dot
        assert_eq!(parse_version_tag("v0.5.0-rc1"), None); // pre-release with dash
        assert_eq!(parse_version_tag("release-2026.04"), None); // mixed alphanumeric
        assert_eq!(parse_version_tag("foo1"), None); // alphanumeric without dot
        assert_eq!(parse_version_tag("invalid"), None); // pure text
        assert_eq!(parse_version_tag("v"), None); // empty after stripping 'v'
        assert_eq!(parse_version_tag("v."), None); // just a dot
    }
}

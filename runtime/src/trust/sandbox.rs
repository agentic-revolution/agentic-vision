//! Input sanitization — block dangerous payloads before ACT execution.

use regex::Regex;
use std::sync::LazyLock;

static SCRIPT_TAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<script[\s>]").unwrap());

static SQL_INJECTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(\b(union|select|insert|update|delete|drop|alter|exec)\b.*\b(from|into|table|where)\b)|(--)|(;.*\b(drop|delete|update)\b)").unwrap()
});

static PATH_TRAVERSAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.\./|\.\.\\").unwrap());

static EVENT_HANDLER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bon\w+\s*=").unwrap());

/// Threats that can be detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Threat {
    ScriptInjection,
    SqlInjection,
    PathTraversal,
    EventHandler,
}

/// Result of sanitization check.
#[derive(Debug)]
pub struct SanitizeResult {
    pub safe: bool,
    pub threats: Vec<Threat>,
}

/// Check if an input value is safe to use in ACT execution.
pub fn check(value: &str) -> SanitizeResult {
    let mut threats = Vec::new();

    if SCRIPT_TAG_RE.is_match(value) {
        threats.push(Threat::ScriptInjection);
    }
    if SQL_INJECTION_RE.is_match(value) {
        threats.push(Threat::SqlInjection);
    }
    if PATH_TRAVERSAL_RE.is_match(value) {
        threats.push(Threat::PathTraversal);
    }
    if EVENT_HANDLER_RE.is_match(value) {
        threats.push(Threat::EventHandler);
    }

    SanitizeResult {
        safe: threats.is_empty(),
        threats,
    }
}

/// Sanitize a string by removing dangerous patterns.
pub fn sanitize(value: &str) -> String {
    let mut result = SCRIPT_TAG_RE.replace_all(value, "").to_string();
    result = EVENT_HANDLER_RE.replace_all(&result, "").to_string();
    result = PATH_TRAVERSAL_RE.replace_all(&result, "").to_string();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_input() {
        let result = check("John Doe");
        assert!(result.safe);
    }

    #[test]
    fn test_script_injection() {
        let result = check("<script>alert(1)</script>");
        assert!(!result.safe);
        assert!(result.threats.contains(&Threat::ScriptInjection));
    }

    #[test]
    fn test_path_traversal() {
        let result = check("../../etc/passwd");
        assert!(!result.safe);
        assert!(result.threats.contains(&Threat::PathTraversal));
    }

    #[test]
    fn test_sanitize() {
        let clean = sanitize("<script>alert(1)</script>hello");
        assert!(!clean.contains("<script"));
    }
}

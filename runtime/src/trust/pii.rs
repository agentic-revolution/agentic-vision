//! PII detection — scan content for sensitive information patterns.

use regex::Regex;
use std::sync::LazyLock;

/// Types of PII that can be detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PiiType {
    Email,
    Phone,
    Ssn,
    CreditCard,
}

/// A detected PII occurrence.
#[derive(Debug, Clone)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub start: usize,
    pub end: usize,
}

static EMAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap());

static PHONE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}").unwrap());

static SSN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap());

static CC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").unwrap());

/// Scan text for PII patterns.
pub fn scan(text: &str) -> Vec<PiiMatch> {
    let mut matches = Vec::new();

    for m in EMAIL_RE.find_iter(text) {
        matches.push(PiiMatch {
            pii_type: PiiType::Email,
            start: m.start(),
            end: m.end(),
        });
    }

    for m in PHONE_RE.find_iter(text) {
        matches.push(PiiMatch {
            pii_type: PiiType::Phone,
            start: m.start(),
            end: m.end(),
        });
    }

    for m in SSN_RE.find_iter(text) {
        matches.push(PiiMatch {
            pii_type: PiiType::Ssn,
            start: m.start(),
            end: m.end(),
        });
    }

    for m in CC_RE.find_iter(text) {
        matches.push(PiiMatch {
            pii_type: PiiType::CreditCard,
            start: m.start(),
            end: m.end(),
        });
    }

    matches.sort_by_key(|m| m.start);
    matches
}

/// Check if text contains any PII.
pub fn has_pii(text: &str) -> bool {
    EMAIL_RE.is_match(text)
        || PHONE_RE.is_match(text)
        || SSN_RE.is_match(text)
        || CC_RE.is_match(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_email() {
        let matches = scan("Contact us at hello@example.com for info");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Email);
    }

    #[test]
    fn test_detect_ssn() {
        let matches = scan("SSN: 123-45-6789");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Ssn);
    }

    #[test]
    fn test_detect_credit_card() {
        let matches = scan("Card: 4111-1111-1111-1111");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::CreditCard);
    }

    #[test]
    fn test_no_pii() {
        assert!(!has_pii("Hello, this is a normal text."));
    }
}

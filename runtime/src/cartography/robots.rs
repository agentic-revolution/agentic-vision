//! Parse robots.txt files.

/// Parsed robots.txt rules.
#[derive(Debug, Clone, Default)]
pub struct RobotsRules {
    pub allowed: Vec<String>,
    pub disallowed: Vec<String>,
    pub crawl_delay: Option<f32>,
    pub sitemaps: Vec<String>,
}

impl RobotsRules {
    /// Check if a path is allowed by the robots rules.
    pub fn is_allowed(&self, path: &str) -> bool {
        // Check disallowed first (more specific wins)
        let mut longest_disallow = 0;
        let mut is_disallowed = false;
        for pattern in &self.disallowed {
            if path_matches(path, pattern) && pattern.len() > longest_disallow {
                longest_disallow = pattern.len();
                is_disallowed = true;
            }
        }

        let mut longest_allow = 0;
        let mut is_allowed = false;
        for pattern in &self.allowed {
            if path_matches(path, pattern) && pattern.len() > longest_allow {
                longest_allow = pattern.len();
                is_allowed = true;
            }
        }

        // Longer match wins
        if is_allowed && is_disallowed {
            return longest_allow >= longest_disallow;
        }
        if is_disallowed {
            return false;
        }
        true
    }
}

/// Parse a robots.txt string for a specific user agent.
pub fn parse_robots(txt: &str, user_agent: &str) -> RobotsRules {
    let mut rules = RobotsRules::default();
    let mut in_matching_group = false;
    let mut found_matching_group = false;
    let ua_lower = user_agent.to_lowercase();

    for line in txt.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Remove inline comments
        let line = line.split('#').next().unwrap_or("").trim();

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim().to_lowercase();
            let value = value.trim();

            match key.as_str() {
                "user-agent" => {
                    let ua = value.to_lowercase();
                    in_matching_group = ua == "*" || ua == ua_lower;
                    if in_matching_group {
                        found_matching_group = true;
                    }
                }
                "allow" if in_matching_group || !found_matching_group => {
                    if !value.is_empty() {
                        rules.allowed.push(value.to_string());
                    }
                }
                "disallow" if in_matching_group || !found_matching_group => {
                    if !value.is_empty() {
                        rules.disallowed.push(value.to_string());
                    }
                }
                "crawl-delay" if in_matching_group || !found_matching_group => {
                    if let Ok(delay) = value.parse::<f32>() {
                        rules.crawl_delay = Some(delay);
                    }
                }
                "sitemap" => {
                    // Sitemap directives are global
                    if !value.is_empty() {
                        rules.sitemaps.push(value.to_string());
                    }
                }
                _ => {}
            }
        }
    }

    rules
}

/// Check if a path matches a robots.txt pattern.
fn path_matches(path: &str, pattern: &str) -> bool {
    if pattern.is_empty() {
        return false;
    }

    // Simple prefix matching (robots.txt standard)
    if let Some(prefix) = pattern.strip_suffix('*') {
        return path.starts_with(prefix);
    }

    if let Some(exact) = pattern.strip_suffix('$') {
        return path == exact;
    }

    path.starts_with(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_robots() {
        let txt = r#"
User-agent: *
Allow: /
Disallow: /admin
Disallow: /private/
Crawl-delay: 1.5

Sitemap: https://example.com/sitemap.xml
Sitemap: https://example.com/sitemap-blog.xml
"#;

        let rules = parse_robots(txt, "cortex");
        assert_eq!(rules.allowed.len(), 1);
        assert_eq!(rules.disallowed.len(), 2);
        assert_eq!(rules.crawl_delay, Some(1.5));
        assert_eq!(rules.sitemaps.len(), 2);

        assert!(rules.is_allowed("/"));
        assert!(rules.is_allowed("/about"));
        assert!(!rules.is_allowed("/admin"));
        assert!(!rules.is_allowed("/admin/settings"));
        assert!(!rules.is_allowed("/private/data"));
    }

    #[test]
    fn test_allow_overrides_disallow() {
        let txt = r#"
User-agent: *
Disallow: /api/
Allow: /api/public/
"#;
        let rules = parse_robots(txt, "cortex");
        assert!(!rules.is_allowed("/api/secret"));
        assert!(rules.is_allowed("/api/public/docs"));
    }
}

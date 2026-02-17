//! Smart sampling: choose which pages to render.

use crate::map::types::PageType;
use std::collections::HashMap;

/// Select a subset of URLs for browser rendering.
///
/// Strategy:
/// - Always include root/home page
/// - Ensure at least 2 samples per PageType present
/// - Fill remaining budget proportionally by PageType frequency
/// - Prefer higher-confidence classifications
pub fn select_samples(
    classified_urls: &[(String, PageType, f32)],
    max_render: usize,
) -> Vec<String> {
    if classified_urls.is_empty() {
        return Vec::new();
    }

    let budget = max_render.min(classified_urls.len());
    let mut selected = Vec::with_capacity(budget);
    let mut selected_set = std::collections::HashSet::new();

    // 1. Always include the home page
    for (url, pt, _conf) in classified_urls {
        if *pt == PageType::Home {
            selected_set.insert(url.clone());
            selected.push(url.clone());
            break;
        }
    }

    // If no explicit home, add the first URL (likely the root)
    if selected.is_empty() {
        let url = &classified_urls[0].0;
        selected_set.insert(url.clone());
        selected.push(url.clone());
    }

    // 2. Group by PageType, sorted by confidence descending
    let mut by_type: HashMap<PageType, Vec<(String, f32)>> = HashMap::new();
    for (url, pt, conf) in classified_urls {
        by_type
            .entry(*pt)
            .or_default()
            .push((url.clone(), *conf));
    }
    for entries in by_type.values_mut() {
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }

    // 3. Ensure at least 2 samples per PageType (if available and budget allows)
    let min_per_type = 2;
    for entries in by_type.values() {
        for (url, _conf) in entries.iter().take(min_per_type) {
            if selected.len() >= budget {
                return selected;
            }
            if selected_set.insert(url.clone()) {
                selected.push(url.clone());
            }
        }
    }

    // 4. Fill remaining budget proportionally by PageType frequency
    if selected.len() < budget {
        let remaining = budget - selected.len();
        let total = classified_urls.len() as f32;

        // Compute proportional allocation
        let mut allocations: Vec<(PageType, usize)> = by_type
            .iter()
            .map(|(&pt, entries)| {
                let proportion = entries.len() as f32 / total;
                let alloc = (proportion * remaining as f32).ceil() as usize;
                (pt, alloc)
            })
            .collect();
        allocations.sort_by(|a, b| b.1.cmp(&a.1));

        for (pt, alloc) in &allocations {
            if let Some(entries) = by_type.get(pt) {
                for (url, _conf) in entries {
                    if selected.len() >= budget {
                        return selected;
                    }
                    if *alloc == 0 {
                        break;
                    }
                    if selected_set.insert(url.clone()) {
                        selected.push(url.clone());
                    }
                }
            }
        }
    }

    selected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_samples_basic() {
        let urls = vec![
            ("https://shop.com/".to_string(), PageType::Home, 0.9),
            (
                "https://shop.com/p/1".to_string(),
                PageType::ProductDetail,
                0.8,
            ),
            (
                "https://shop.com/p/2".to_string(),
                PageType::ProductDetail,
                0.8,
            ),
            (
                "https://shop.com/p/3".to_string(),
                PageType::ProductDetail,
                0.7,
            ),
            (
                "https://shop.com/about".to_string(),
                PageType::AboutPage,
                0.85,
            ),
            (
                "https://shop.com/blog/1".to_string(),
                PageType::Article,
                0.75,
            ),
        ];

        let samples = select_samples(&urls, 4);
        assert_eq!(samples.len(), 4);
        // Home page should always be first
        assert_eq!(samples[0], "https://shop.com/");
    }

    #[test]
    fn test_select_samples_max_budget() {
        let urls = vec![
            ("https://example.com/".to_string(), PageType::Home, 0.9),
            (
                "https://example.com/a".to_string(),
                PageType::Article,
                0.8,
            ),
        ];

        // Budget larger than URL count
        let samples = select_samples(&urls, 100);
        assert_eq!(samples.len(), 2);
    }

    #[test]
    fn test_select_samples_empty() {
        let samples = select_samples(&[], 10);
        assert!(samples.is_empty());
    }
}

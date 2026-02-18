//! Discovers drag-and-drop interactions and maps them to their underlying API calls.
//!
//! Instead of simulating mouse drags in a browser, this module identifies the
//! HTTP endpoint that persists the drag result. Three discovery strategies:
//!
//! 1. **Library detection** -- recognise react-beautiful-dnd, SortableJS, Angular CDK,
//!    dnd-kit, jQuery UI, and HTML5 native drag from DOM attributes and JS signatures.
//! 2. **Element discovery** -- find draggable elements, drop zones, and their data attributes.
//! 3. **API extraction** -- scan JS bundles for the fetch/axios/XHR call made after a drop.
//!
//! All public entry points are **synchronous**. Callers should wrap in
//! `tokio::task::spawn_blocking` when integrating with the async runtime.

use crate::map::types::OpCode;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

// ── Compile-time platform configuration ─────────────────────────────────────

/// Raw JSON content of the drag platform templates, embedded at compile time.
const DRAG_PLATFORMS_JSON: &str = include_str!("drag_platforms.json");

// ── Public types ────────────────────────────────────────────────────────────

/// Which drag-and-drop library is in use on the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DragLibrary {
    /// Facebook's react-beautiful-dnd library.
    ReactBeautifulDnd,
    /// SortableJS / Sortable library.
    SortableJS,
    /// Angular CDK drag-and-drop module.
    AngularCdk,
    /// dnd-kit (React-based drag-and-drop toolkit).
    DndKit,
    /// jQuery UI Sortable / Draggable.
    JQueryUI,
    /// Native HTML5 drag-and-drop API.
    Html5Native,
    /// No recognised drag-and-drop library.
    Unknown,
}

/// An API endpoint discovered from JavaScript analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// The endpoint URL (may contain path templates like `{id}`).
    pub url: String,
    /// HTTP method: GET, POST, PUT, PATCH, or DELETE.
    pub method: String,
    /// Optional body template extracted from JS source.
    pub body_template: Option<String>,
}

/// A discovered drag-and-drop interaction that can be replayed via HTTP.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragAction {
    /// The drag-and-drop library detected on the page.
    pub drag_library: DragLibrary,
    /// CSS selector for draggable elements.
    pub draggable_selector: String,
    /// Data attribute on draggable elements that holds the source item ID.
    pub source_id_attr: String,
    /// CSS selector for drop zone elements.
    pub drop_zone_selector: String,
    /// Data attribute on drop zones that holds the target container ID.
    pub target_id_attr: String,
    /// The API endpoint that persists the drag result (if discovered).
    pub api_endpoint: Option<ApiEndpoint>,
    /// Query/body parameter name for the new position (e.g., `"pos"`, `"index"`).
    pub position_param: String,
    /// OpCode for this drag action in the binary map spec.
    pub opcode: OpCode,
    /// Confidence that this drag interaction is correctly identified, in `[0.0, 1.0]`.
    pub confidence: f32,
}

// ── Internal JSON deserialization types ─────────────────────────────────────

/// Known drag-and-drop configuration for a specific platform domain.
#[derive(Debug, Clone, Deserialize)]
struct PlatformDragConfig {
    /// The type of drag interaction (e.g., `"card_move"`, `"task_move"`).
    drag_type: String,
    /// API endpoint details for persisting the drag result.
    api: PlatformDragApi,
    /// CSS selector for source (draggable) elements.
    source_selector: String,
    /// Data attribute name that holds the source element ID.
    source_id: Option<String>,
    /// CSS selector for target (drop zone) elements.
    target_selector: Option<String>,
    /// Data attribute name that holds the target zone ID.
    target_id: Option<String>,
}

/// API configuration for a platform's drag-and-drop endpoint.
#[derive(Debug, Clone, Deserialize)]
struct PlatformDragApi {
    /// HTTP method (e.g., `"PUT"`, `"POST"`, `"PATCH"`).
    method: String,
    /// URL path template (e.g., `"/1/cards/{card_id}"`).
    path: String,
    /// Optional body template as a JSON value.
    body: Option<serde_json::Value>,
}

type DragPlatformRegistry = std::collections::HashMap<String, PlatformDragConfig>;

/// Parse and cache the embedded drag platform templates.
fn drag_platform_registry() -> &'static DragPlatformRegistry {
    static REGISTRY: OnceLock<DragPlatformRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| serde_json::from_str(DRAG_PLATFORMS_JSON).unwrap_or_default())
}

// ── Public API ──────────────────────────────────────────────────────────────

/// Discover drag-and-drop interactions from HTML and JavaScript bundles.
///
/// This is the main entry point. It layers three discovery strategies:
///
/// 1. **Platform registry** -- checks if the page belongs to a known platform
///    (Trello, GitHub Projects, Asana, etc.) and returns pre-configured actions.
/// 2. **Library detection** -- identifies the drag-and-drop library from DOM
///    attributes and JS signatures.
/// 3. **API extraction** -- scans JS bundles for the HTTP call made after a drop.
///
/// # Arguments
///
/// * `html` -- raw HTML source of the page.
/// * `js_bundles` -- JavaScript source texts loaded by the page.
///
/// # Returns
///
/// A vector of [`DragAction`] items, one per discovered drag interaction.
pub fn discover_drag_actions(html: &str, js_bundles: &[String]) -> Vec<DragAction> {
    let mut actions = Vec::new();

    // Strategy 1: check platform registry for known domains.
    if let Some(domain) = extract_domain_from_html(html) {
        let platform_actions = discover_drag_from_platform(&domain);
        if !platform_actions.is_empty() {
            return platform_actions;
        }
    }

    // Strategy 2: detect drag library from HTML attributes and JS signatures.
    let library = detect_drag_library(html, js_bundles);
    if library == DragLibrary::Unknown {
        return actions;
    }

    // Strategy 3: find draggable elements, drop zones, and API endpoint.
    let (draggable_selector, source_id_attr, drop_zone_selector, target_id_attr) =
        find_drag_elements(html, &library);

    let api_endpoint = scan_js_for_drag_api(js_bundles);

    let confidence = compute_confidence(&library, &api_endpoint, &draggable_selector);

    actions.push(DragAction {
        drag_library: library,
        draggable_selector,
        source_id_attr,
        drop_zone_selector,
        target_id_attr,
        api_endpoint,
        position_param: "position".to_string(),
        opcode: OpCode::new(0x07, 0x00),
        confidence,
    });

    actions
}

/// Detect which drag-and-drop library is in use on the page.
///
/// Checks HTML for characteristic DOM attributes and CSS classes, then
/// scans JavaScript bundles for library-specific function signatures.
///
/// # Detection rules
///
/// | Signal | Library |
/// |--------|---------|
/// | `[data-rbd-draggable-id]` in HTML | react-beautiful-dnd |
/// | `[cdkDrag]` in HTML | Angular CDK |
/// | `.sortable` class in HTML | SortableJS |
/// | `.ui-sortable` class in HTML | jQuery UI |
/// | `[draggable="true"]` in HTML | HTML5 Native |
/// | `Sortable.create` in JS | SortableJS |
/// | `DragDropContext` in JS | react-beautiful-dnd |
/// | `useDraggable` or `@dnd-kit` in JS | dnd-kit |
/// | `cdkDrag` in JS | Angular CDK |
///
/// # Arguments
///
/// * `html` -- raw HTML source of the page.
/// * `js_bundles` -- JavaScript source texts loaded by the page.
///
/// # Returns
///
/// The detected [`DragLibrary`] variant, or [`DragLibrary::Unknown`].
pub fn detect_drag_library(html: &str, js_bundles: &[String]) -> DragLibrary {
    // Check HTML DOM attributes first (more specific signals).
    let document = Html::parse_document(html);

    // react-beautiful-dnd: data-rbd-draggable-id
    if let Ok(sel) = Selector::parse("[data-rbd-draggable-id]") {
        if document.select(&sel).next().is_some() {
            return DragLibrary::ReactBeautifulDnd;
        }
    }

    // Angular CDK: cdkDrag attribute
    if let Ok(sel) = Selector::parse("[cdkDrag], [cdkdrag]") {
        if document.select(&sel).next().is_some() {
            return DragLibrary::AngularCdk;
        }
    }

    // jQuery UI: .ui-sortable class
    if let Ok(sel) = Selector::parse(".ui-sortable") {
        if document.select(&sel).next().is_some() {
            return DragLibrary::JQueryUI;
        }
    }

    // SortableJS: .sortable class (check after jQuery UI to avoid false positives)
    if let Ok(sel) = Selector::parse(".sortable") {
        if document.select(&sel).next().is_some() {
            return DragLibrary::SortableJS;
        }
    }

    // Check JS bundles for library signatures.
    let js_combined: String = js_bundles
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if js_combined.contains("DragDropContext") || js_combined.contains("data-rbd-draggable-id") {
        return DragLibrary::ReactBeautifulDnd;
    }

    if js_combined.contains("useDraggable") || js_combined.contains("@dnd-kit") {
        return DragLibrary::DndKit;
    }

    if js_combined.contains("Sortable.create") || js_combined.contains("new Sortable") {
        return DragLibrary::SortableJS;
    }

    if js_combined.contains("cdkDrag") || js_combined.contains("CdkDragDrop") {
        return DragLibrary::AngularCdk;
    }

    // HTML5 native: draggable="true" (least specific, check last)
    if let Ok(sel) = Selector::parse("[draggable=\"true\"]") {
        if document.select(&sel).next().is_some() {
            return DragLibrary::Html5Native;
        }
    }

    DragLibrary::Unknown
}

/// Look up a domain in the platform registry and return pre-configured drag actions.
///
/// Known platforms include Trello, GitHub Projects, Asana, Notion, Monday.com,
/// and Jira. Each has pre-mapped API endpoints and selectors embedded in
/// `drag_platforms.json`.
///
/// # Arguments
///
/// * `domain` -- the domain to look up (e.g., `"trello.com"`).
///
/// # Returns
///
/// A vector of [`DragAction`] items from the platform template, or an empty
/// vector if the domain is not recognised.
pub fn discover_drag_from_platform(domain: &str) -> Vec<DragAction> {
    let registry = drag_platform_registry();

    // Try exact match first, then check if the domain ends with a known platform.
    let config = registry.get(domain).or_else(|| {
        registry
            .iter()
            .find(|(key, _)| domain.ends_with(key.as_str()))
            .map(|(_, v)| v)
    });

    let config = match config {
        Some(c) => c,
        None => return Vec::new(),
    };

    let body_template = config
        .api
        .body
        .as_ref()
        .map(|b| serde_json::to_string(b).unwrap_or_default());

    let api_endpoint = ApiEndpoint {
        url: config.api.path.clone(),
        method: config.api.method.clone(),
        body_template,
    };

    vec![DragAction {
        drag_library: DragLibrary::Unknown,
        draggable_selector: config.source_selector.clone(),
        source_id_attr: config.source_id.clone().unwrap_or_default(),
        drop_zone_selector: config.target_selector.clone().unwrap_or_default(),
        target_id_attr: config.target_id.clone().unwrap_or_default(),
        api_endpoint: Some(api_endpoint),
        position_param: "position".to_string(),
        opcode: OpCode::new(0x07, 0x00),
        confidence: 0.95,
    }]
}

// ── Private helpers ─────────────────────────────────────────────────────────

/// Scan JavaScript bundles for the API call made after a drag-and-drop event.
///
/// Looks for function bodies named `onDragEnd`, `handleDrop`, `onSortEnd`, or
/// `dropHandler`, then extracts `fetch()` / `axios` / `$.ajax` calls within.
fn scan_js_for_drag_api(js_bundles: &[String]) -> Option<ApiEndpoint> {
    let js_combined: String = js_bundles
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    // Find the start of each drag handler by name.
    let handler_name_re = Regex::new(
        r"(?:onDragEnd|handleDrop|onSortEnd|dropHandler|onDragStop)\s*(?:=\s*(?:async\s*)?(?:\([^)]*\)|[a-zA-Z_]\w*)\s*=>|[:=]\s*(?:async\s+)?function\s*\([^)]*\))\s*\{"
    ).ok()?;

    // Pre-compile all regexes outside the loop to satisfy clippy.
    let fetch_re = Regex::new(
        r#"fetch\(\s*['"`]([^'"`]+)['"`]\s*(?:,\s*\{[^}]*method\s*:\s*['"`](\w+)['"`])?"#,
    )
    .ok()?;
    let axios_re =
        Regex::new(r#"axios\.(get|post|put|patch|delete)\(\s*['"`]([^'"`]+)['"`]"#).ok()?;
    let ajax_re = Regex::new(r#"\$\.ajax\(\s*\{([^}]*)\}"#).ok()?;
    let url_re = Regex::new(r#"url\s*:\s*['"`]([^'"`]+)['"`]"#).ok()?;
    let type_re = Regex::new(r#"type\s*:\s*['"`](\w+)['"`]"#).ok()?;

    for m in handler_name_re.find_iter(&js_combined) {
        // Extract the function body by counting brace depth from the opening `{`.
        let body = extract_brace_body(&js_combined[m.end()..]);

        // Look for fetch() calls inside the handler body.
        if let Some(fetch_caps) = fetch_re.captures(&body) {
            let url = fetch_caps.get(1).map_or("", |m| m.as_str()).to_string();
            let method = fetch_caps
                .get(2)
                .map_or("POST", |m| m.as_str())
                .to_uppercase();

            let body_template = extract_body_template(&body);

            return Some(ApiEndpoint {
                url,
                method,
                body_template,
            });
        }

        // Look for axios calls inside the handler body.
        if let Some(axios_caps) = axios_re.captures(&body) {
            let method = axios_caps
                .get(1)
                .map_or("POST", |m| m.as_str())
                .to_uppercase();
            let url = axios_caps.get(2).map_or("", |m| m.as_str()).to_string();
            let body_template = extract_body_template(&body);

            return Some(ApiEndpoint {
                url,
                method,
                body_template,
            });
        }

        // Look for $.ajax calls inside the handler body.
        if let Some(ajax_caps) = ajax_re.captures(&body) {
            let ajax_block = ajax_caps.get(1).map_or("", |m| m.as_str());

            if let Some(url_caps) = url_re.captures(ajax_block) {
                let url = url_caps.get(1).map_or("", |m| m.as_str()).to_string();
                let method = type_re
                    .captures(ajax_block)
                    .and_then(|c| c.get(1))
                    .map_or("POST", |m| m.as_str())
                    .to_uppercase();

                return Some(ApiEndpoint {
                    url,
                    method,
                    body_template: None,
                });
            }
        }
    }

    None
}

/// Extract the content between a matched opening `{` and its balancing closing `}`.
///
/// `s` must start right *after* the opening brace. Returns the text between
/// the braces (exclusive), or the full string if no balancing brace is found.
fn extract_brace_body(s: &str) -> String {
    let mut depth: u32 = 1;
    let mut in_string = false;
    let mut string_char: char = '"';
    let mut prev_char = '\0';

    for (i, ch) in s.char_indices() {
        if in_string {
            if ch == string_char && prev_char != '\\' {
                in_string = false;
            }
            prev_char = ch;
            continue;
        }

        match ch {
            '"' | '\'' | '`' => {
                in_string = true;
                string_char = ch;
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return s[..i].to_string();
                }
            }
            _ => {}
        }
        prev_char = ch;
    }

    s.to_string()
}

/// Try to extract a body template from a JS function body.
///
/// Looks for `JSON.stringify(...)` or inline object literals near fetch/axios calls.
fn extract_body_template(js_body: &str) -> Option<String> {
    // Look for JSON.stringify({ ... })
    let stringify_re = Regex::new(r"JSON\.stringify\(\s*(\{[^}]+\})").ok()?;
    if let Some(caps) = stringify_re.captures(js_body) {
        return Some(caps.get(1).map_or("", |m| m.as_str()).to_string());
    }

    // Look for body: { ... }
    let body_obj_re = Regex::new(r"body\s*:\s*(\{[^}]+\})").ok()?;
    if let Some(caps) = body_obj_re.captures(js_body) {
        return Some(caps.get(1).map_or("", |m| m.as_str()).to_string());
    }

    None
}

/// Extract the domain from HTML `<base>`, `<link rel="canonical">`, or `<meta>` tags.
fn extract_domain_from_html(html: &str) -> Option<String> {
    let document = Html::parse_document(html);

    // Try <base href="...">
    if let Ok(sel) = Selector::parse("base[href]") {
        if let Some(el) = document.select(&sel).next() {
            if let Some(href) = el.value().attr("href") {
                if let Some(domain) = domain_from_url(href) {
                    return Some(domain);
                }
            }
        }
    }

    // Try <link rel="canonical" href="...">
    if let Ok(sel) = Selector::parse("link[rel=\"canonical\"]") {
        if let Some(el) = document.select(&sel).next() {
            if let Some(href) = el.value().attr("href") {
                if let Some(domain) = domain_from_url(href) {
                    return Some(domain);
                }
            }
        }
    }

    // Try <meta property="og:url" content="...">
    if let Ok(sel) = Selector::parse("meta[property=\"og:url\"]") {
        if let Some(el) = document.select(&sel).next() {
            if let Some(content) = el.value().attr("content") {
                if let Some(domain) = domain_from_url(content) {
                    return Some(domain);
                }
            }
        }
    }

    None
}

/// Extract the host/domain from a URL string.
fn domain_from_url(url_str: &str) -> Option<String> {
    url::Url::parse(url_str)
        .ok()
        .and_then(|u| u.host_str().map(String::from))
}

/// Find draggable elements and drop zones based on the detected library.
///
/// Returns `(draggable_selector, source_id_attr, drop_zone_selector, target_id_attr)`.
fn find_drag_elements(html: &str, library: &DragLibrary) -> (String, String, String, String) {
    let document = Html::parse_document(html);

    match library {
        DragLibrary::ReactBeautifulDnd => {
            let draggable_sel = "[data-rbd-draggable-id]".to_string();
            let source_id = "data-rbd-draggable-id".to_string();
            let drop_zone_sel = "[data-rbd-droppable-id]".to_string();
            let target_id = "data-rbd-droppable-id".to_string();
            (draggable_sel, source_id, drop_zone_sel, target_id)
        }
        DragLibrary::SortableJS => {
            // SortableJS uses .sortable containers; children are draggable.
            let draggable_sel = ".sortable > *".to_string();
            let source_id = find_data_id_attr(&document, ".sortable > *");
            let drop_zone_sel = ".sortable".to_string();
            let target_id = find_data_id_attr(&document, ".sortable");
            (draggable_sel, source_id, drop_zone_sel, target_id)
        }
        DragLibrary::AngularCdk => {
            let draggable_sel = "[cdkDrag], [cdkdrag]".to_string();
            let source_id = find_data_id_attr(&document, "[cdkDrag], [cdkdrag]");
            let drop_zone_sel = "[cdkDropList], [cdkdroplist]".to_string();
            let target_id = find_data_id_attr(&document, "[cdkDropList], [cdkdroplist]");
            (draggable_sel, source_id, drop_zone_sel, target_id)
        }
        DragLibrary::DndKit => {
            let draggable_sel = "[data-dnd-draggable]".to_string();
            let source_id = "data-dnd-draggable".to_string();
            let drop_zone_sel = "[data-dnd-droppable]".to_string();
            let target_id = "data-dnd-droppable".to_string();
            (draggable_sel, source_id, drop_zone_sel, target_id)
        }
        DragLibrary::JQueryUI => {
            let draggable_sel = ".ui-sortable > *".to_string();
            let source_id = find_data_id_attr(&document, ".ui-sortable > *");
            let drop_zone_sel = ".ui-sortable".to_string();
            let target_id = find_data_id_attr(&document, ".ui-sortable");
            (draggable_sel, source_id, drop_zone_sel, target_id)
        }
        DragLibrary::Html5Native => {
            let draggable_sel = "[draggable=\"true\"]".to_string();
            let source_id = find_data_id_attr(&document, "[draggable=\"true\"]");
            let drop_zone_sel = "[data-drop-zone], [ondrop]".to_string();
            let target_id = find_data_id_attr(&document, "[data-drop-zone], [ondrop]");
            (draggable_sel, source_id, drop_zone_sel, target_id)
        }
        DragLibrary::Unknown => (
            "".to_string(),
            "".to_string(),
            "".to_string(),
            "".to_string(),
        ),
    }
}

/// Try to find a `data-*-id` attribute on the first element matching a selector.
fn find_data_id_attr(document: &Html, selector_str: &str) -> String {
    let sel = match Selector::parse(selector_str) {
        Ok(s) => s,
        Err(_) => return "id".to_string(),
    };

    if let Some(el) = document.select(&sel).next() {
        // Look for data-*-id or data-id attributes.
        for attr in el.value().attrs() {
            let (name, _) = attr;
            if name.starts_with("data-") && (name.ends_with("-id") || name == "data-id") {
                return name.to_string();
            }
        }
        // Fall back to "id" if the element has an id attribute.
        if el.value().attr("id").is_some() {
            return "id".to_string();
        }
    }

    "id".to_string()
}

/// Compute confidence based on the quality of discovery signals.
fn compute_confidence(
    library: &DragLibrary,
    api_endpoint: &Option<ApiEndpoint>,
    draggable_selector: &str,
) -> f32 {
    let mut confidence = 0.0f32;

    // Known library adds base confidence.
    confidence += match library {
        DragLibrary::ReactBeautifulDnd => 0.40,
        DragLibrary::SortableJS => 0.35,
        DragLibrary::AngularCdk => 0.35,
        DragLibrary::DndKit => 0.35,
        DragLibrary::JQueryUI => 0.30,
        DragLibrary::Html5Native => 0.20,
        DragLibrary::Unknown => 0.0,
    };

    // Found API endpoint adds significant confidence.
    if api_endpoint.is_some() {
        confidence += 0.40;
    }

    // Found draggable elements in DOM adds confidence.
    if !draggable_selector.is_empty() {
        confidence += 0.15;
    }

    confidence.min(1.0)
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_react_beautiful_dnd() {
        let html = r#"
        <html>
        <body>
            <div data-rbd-droppable-id="list-1">
                <div data-rbd-draggable-id="item-1" data-rbd-drag-handle-draggable-id="item-1">
                    <span>Task 1</span>
                </div>
                <div data-rbd-draggable-id="item-2" data-rbd-drag-handle-draggable-id="item-2">
                    <span>Task 2</span>
                </div>
            </div>
        </body>
        </html>
        "#;

        let library = detect_drag_library(html, &[]);
        assert_eq!(library, DragLibrary::ReactBeautifulDnd);

        // Also test via JS detection.
        let js =
            vec!["import { DragDropContext, Droppable } from 'react-beautiful-dnd';".to_string()];
        let library_js = detect_drag_library("<html><body></body></html>", &js);
        assert_eq!(library_js, DragLibrary::ReactBeautifulDnd);
    }

    #[test]
    fn test_detect_sortablejs() {
        let html = r#"
        <html>
        <body>
            <ul class="sortable" id="task-list">
                <li data-item-id="1">Item 1</li>
                <li data-item-id="2">Item 2</li>
                <li data-item-id="3">Item 3</li>
            </ul>
        </body>
        </html>
        "#;

        let js = vec!["var sortable = Sortable.create(document.getElementById('task-list'), { animation: 150 });".to_string()];

        let library = detect_drag_library(html, &js);
        assert_eq!(library, DragLibrary::SortableJS);
    }

    #[test]
    fn test_discover_drag_from_platform_trello() {
        let actions = discover_drag_from_platform("trello.com");
        assert!(!actions.is_empty());

        let action = &actions[0];
        assert_eq!(action.opcode, OpCode::new(0x07, 0x00));
        assert!(action.confidence > 0.0);
        assert!(action.api_endpoint.is_some());

        let api = action.api_endpoint.as_ref().unwrap();
        assert_eq!(api.method, "PUT");
        assert!(api.url.contains("/cards/"));
    }

    #[test]
    fn test_scan_js_for_drag_api() {
        let js_with_handler = r#"
            const onDragEnd = async (result) => {
                if (!result.destination) return;
                const { source, destination } = result;
                await fetch('/api/reorder', {
                    method: 'PUT',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ itemId: source.index, newPosition: destination.index })
                });
            };
        "#;

        let bundles = vec![js_with_handler.to_string()];
        let endpoint = scan_js_for_drag_api(&bundles);
        assert!(endpoint.is_some());

        let ep = endpoint.unwrap();
        assert_eq!(ep.url, "/api/reorder");
        assert_eq!(ep.method, "PUT");
        assert!(ep.body_template.is_some());
    }

    #[test]
    fn test_empty_html() {
        let actions = discover_drag_actions("", &[]);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_detect_angular_cdk() {
        let html = r#"
        <html>
        <body>
            <div cdkDropList>
                <div cdkDrag data-item-id="a1">Item A</div>
                <div cdkDrag data-item-id="a2">Item B</div>
            </div>
        </body>
        </html>
        "#;

        let library = detect_drag_library(html, &[]);
        assert_eq!(library, DragLibrary::AngularCdk);
    }

    #[test]
    fn test_detect_jquery_ui() {
        let html = r#"
        <html>
        <body>
            <ul class="ui-sortable">
                <li class="ui-sortable-handle" data-task-id="t1">Task 1</li>
                <li class="ui-sortable-handle" data-task-id="t2">Task 2</li>
            </ul>
        </body>
        </html>
        "#;

        let library = detect_drag_library(html, &[]);
        assert_eq!(library, DragLibrary::JQueryUI);
    }

    #[test]
    fn test_detect_dnd_kit_from_js() {
        let js = vec!["import { useDraggable, useDroppable } from '@dnd-kit/core';".to_string()];
        let library = detect_drag_library("<html><body></body></html>", &js);
        assert_eq!(library, DragLibrary::DndKit);
    }

    #[test]
    fn test_detect_html5_native() {
        let html = r#"
        <html>
        <body>
            <div draggable="true" data-item-id="x1">Drag me</div>
            <div draggable="true" data-item-id="x2">Drag me too</div>
            <div data-drop-zone="zone-1" ondrop="handleDrop(event)">Drop here</div>
        </body>
        </html>
        "#;

        let library = detect_drag_library(html, &[]);
        assert_eq!(library, DragLibrary::Html5Native);
    }

    #[test]
    fn test_discover_drag_from_platform_unknown() {
        let actions = discover_drag_from_platform("unknown-site.example.org");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_discover_drag_actions_with_rbd_and_api() {
        let html = r#"
        <html>
        <head><link rel="canonical" href="https://myapp.example.com/board" /></head>
        <body>
            <div data-rbd-droppable-id="col-1">
                <div data-rbd-draggable-id="card-1">Card 1</div>
                <div data-rbd-draggable-id="card-2">Card 2</div>
            </div>
        </body>
        </html>
        "#;

        let js = vec![r#"
            const onDragEnd = (result) => {
                fetch('/api/cards/reorder', {
                    method: 'POST',
                    body: JSON.stringify({ cardId: result.draggableId, column: result.destination.droppableId })
                });
            };
        "#.to_string()];

        let actions = discover_drag_actions(html, &js);
        assert!(!actions.is_empty());

        let action = &actions[0];
        assert_eq!(action.drag_library, DragLibrary::ReactBeautifulDnd);
        assert_eq!(action.draggable_selector, "[data-rbd-draggable-id]");
        assert_eq!(action.drop_zone_selector, "[data-rbd-droppable-id]");
        assert!(action.api_endpoint.is_some());
        assert!(action.confidence > 0.5);
    }

    #[test]
    fn test_extract_domain_from_html() {
        let html = r#"
        <html>
        <head>
            <base href="https://trello.com/b/abc123" />
        </head>
        <body></body>
        </html>
        "#;

        let domain = extract_domain_from_html(html);
        assert_eq!(domain, Some("trello.com".to_string()));
    }

    #[test]
    fn test_compute_confidence_ranges() {
        // Known library + API endpoint should be high confidence.
        let high = compute_confidence(
            &DragLibrary::ReactBeautifulDnd,
            &Some(ApiEndpoint {
                url: "/api/reorder".to_string(),
                method: "POST".to_string(),
                body_template: None,
            }),
            "[data-rbd-draggable-id]",
        );
        assert!(high >= 0.90, "expected >= 0.90, got {high}");

        // Unknown library with no API should be zero.
        let low = compute_confidence(&DragLibrary::Unknown, &None, "");
        assert!((low - 0.0).abs() < f32::EPSILON, "expected 0.0, got {low}");
    }
}

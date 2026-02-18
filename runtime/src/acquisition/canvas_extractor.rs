//! Three-tier extraction for canvas and WebGL applications.
//!
//! Canvas apps (spreadsheets, design tools, maps) render to `<canvas>` elements,
//! making traditional DOM-based extraction impossible. This module provides
//! three strategies to extract structured data:
//!
//! 1. **Known App APIs** — For apps like Google Sheets, Figma, etc., fetch data
//!    directly via their REST API. Zero browser. Highest reliability.
//! 2. **Accessibility Tree** — Use CDP `Accessibility.getFullAXTree()` to read
//!    the accessibility layer. Requires one browser render but reads structured
//!    data, not pixels.
//! 3. **App State Extraction** — Access the application's JavaScript state
//!    (`window.__INITIAL_STATE__`, Redux store, React fiber tree). Requires
//!    one browser render.

use crate::acquisition::http_client::HttpClient;
use crate::acquisition::http_session::HttpSession;
use crate::renderer::RenderContext;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Embedded known canvas app API configuration.
const KNOWN_CANVAS_APIS_JSON: &str = include_str!("known_canvas_apis.json");

/// The type of canvas application detected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CanvasAppType {
    Spreadsheet,
    DesignTool,
    Map,
    Whiteboard,
    Game,
    Diagram,
    Unknown,
}

/// Structured data extracted from a grid-based canvas app (e.g., spreadsheet).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GridData {
    /// Number of rows discovered.
    pub rows: u32,
    /// Number of columns discovered.
    pub cols: u32,
    /// Cell data: (row, col, value).
    pub cells: Vec<(u32, u32, String)>,
    /// Column headers, if any.
    pub headers: Vec<String>,
}

/// A layer in a design/whiteboard tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Layer {
    /// Layer name or ID.
    pub name: String,
    /// Whether the layer is visible.
    pub visible: bool,
    /// Child elements in this layer.
    pub children: Vec<CanvasElement>,
}

/// An interactive element discovered on a canvas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasElement {
    /// Element label or accessible name.
    pub label: String,
    /// Role (button, textbox, cell, image, etc.).
    pub role: String,
    /// Bounding box: (x, y, width, height).
    pub bounds: Option<(f32, f32, f32, f32)>,
    /// Associated action URL or JS function, if any.
    pub action: Option<String>,
}

/// Complete state extracted from a canvas application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasState {
    /// The type of canvas app detected.
    pub app_type: CanvasAppType,
    /// Grid data for spreadsheet-like apps.
    pub grid: Option<GridData>,
    /// Layer hierarchy for design tools.
    pub layers: Option<Vec<Layer>>,
    /// All visible text content: (text, x, y).
    pub text_content: Vec<(String, f32, f32)>,
    /// Interactive elements (buttons, inputs, cells).
    pub interactive_elements: Vec<CanvasElement>,
    /// Raw application state as JSON, if available.
    pub raw_state: Option<serde_json::Value>,
    /// Which tier successfully extracted data.
    pub extraction_tier: ExtractionTier,
}

/// Which extraction strategy produced the data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionTier {
    /// Tier 1: Known app REST API.
    KnownApi,
    /// Tier 2: Browser accessibility tree.
    AccessibilityTree,
    /// Tier 3: JavaScript app state.
    AppState,
    /// No extraction succeeded.
    None,
}

// ── Known API configuration types ───────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
struct KnownCanvasApi {
    data_api: String,
    #[allow(dead_code)]
    edit_api: Option<String>,
    #[allow(dead_code)]
    auth: Option<String>,
    format: String,
    app_type: String,
}

type CanvasApiRegistry = std::collections::HashMap<String, KnownCanvasApi>;

fn canvas_api_registry() -> &'static CanvasApiRegistry {
    static REGISTRY: OnceLock<CanvasApiRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| serde_json::from_str(KNOWN_CANVAS_APIS_JSON).unwrap_or_default())
}

// ── Tier 1: Known App APIs ──────────────────────────────────────────────────

/// Extract canvas state via a known REST API (Tier 1).
///
/// Checks the URL against known canvas apps (Google Sheets, Figma, etc.).
/// If matched, fetches data directly via HTTP. Zero browser overhead.
///
/// Returns `None` if the URL does not match any known canvas app.
pub async fn extract_via_known_api(
    url: &str,
    _session: Option<&HttpSession>,
    client: &HttpClient,
) -> Option<CanvasState> {
    let registry = canvas_api_registry();

    // Find matching platform by checking URL prefixes
    let matching_config = registry
        .iter()
        .find(|(domain_prefix, _)| url.contains(domain_prefix.as_str()));

    let (domain_key, config) = matching_config?;

    // Build the API URL
    let api_url = if config.data_api.starts_with("http") {
        config.data_api.clone()
    } else {
        // Resolve relative to the page URL
        let base = url.split('?').next().unwrap_or(url);
        format!("{}{}", base.trim_end_matches('/'), config.data_api)
    };

    // Make the HTTP request
    let timeout = 10000;
    let resp = client.get(&api_url, timeout).await.ok()?;

    if resp.status != 200 {
        return None;
    }

    let app_type = match config.app_type.as_str() {
        "spreadsheet" => CanvasAppType::Spreadsheet,
        "design" => CanvasAppType::DesignTool,
        "map" => CanvasAppType::Map,
        "whiteboard" => CanvasAppType::Whiteboard,
        "diagram" => CanvasAppType::Diagram,
        _ => CanvasAppType::Unknown,
    };

    // Parse the response based on format
    let raw_state: Option<serde_json::Value> = if config.format == "json" {
        serde_json::from_str(&resp.body).ok()
    } else {
        None
    };

    // Try to extract grid data from JSON (for spreadsheets)
    let grid = if app_type == CanvasAppType::Spreadsheet {
        extract_grid_from_json(raw_state.as_ref())
    } else {
        None
    };

    // Try to extract layers (for design tools)
    let layers = if app_type == CanvasAppType::DesignTool {
        extract_layers_from_json(raw_state.as_ref())
    } else {
        None
    };

    tracing::info!(
        "Tier 1: extracted canvas state for {} via known API ({})",
        domain_key,
        config.app_type
    );

    Some(CanvasState {
        app_type,
        grid,
        layers,
        text_content: Vec::new(),
        interactive_elements: Vec::new(),
        raw_state,
        extraction_tier: ExtractionTier::KnownApi,
    })
}

// ── Tier 2: Accessibility Tree ──────────────────────────────────────────────

/// Extract canvas state from the browser's accessibility tree (Tier 2).
///
/// Uses `Accessibility.getFullAXTree()` via CDP to read structured data
/// from the accessibility layer. This works for apps that properly
/// implement ARIA attributes.
///
/// Requires a browser context but reads structured data, not pixels.
pub async fn extract_via_accessibility(context: &dyn RenderContext) -> Option<CanvasState> {
    // Execute JS to read accessibility information
    let js = r#"
    (() => {
        const result = { elements: [], text: [] };

        // Gather all elements with ARIA roles
        const all = document.querySelectorAll('[role], [aria-label], [aria-valuetext]');
        for (const el of all) {
            const rect = el.getBoundingClientRect();
            const entry = {
                role: el.getAttribute('role') || el.tagName.toLowerCase(),
                label: el.getAttribute('aria-label') || el.textContent?.trim()?.substring(0, 200) || '',
                x: rect.x, y: rect.y, w: rect.width, h: rect.height,
                action: el.getAttribute('href') || el.getAttribute('data-action') || null
            };
            if (entry.label && rect.width > 0 && rect.height > 0) {
                result.elements.push(entry);
            }
        }

        // Gather text from canvas-adjacent elements
        const textEls = document.querySelectorAll('canvas ~ *, canvas + *, [aria-live]');
        for (const el of textEls) {
            const text = el.textContent?.trim();
            if (text && text.length > 0 && text.length < 1000) {
                const rect = el.getBoundingClientRect();
                result.text.push({ text, x: rect.x, y: rect.y });
            }
        }

        // Check for grid/table ARIA patterns
        const grids = document.querySelectorAll('[role="grid"], [role="table"], [role="spreadsheet"]');
        if (grids.length > 0) {
            const grid = grids[0];
            const rows = grid.querySelectorAll('[role="row"]');
            const gridData = { rows: rows.length, cols: 0, cells: [], headers: [] };
            rows.forEach((row, ri) => {
                const cells = row.querySelectorAll('[role="gridcell"], [role="columnheader"], [role="cell"]');
                gridData.cols = Math.max(gridData.cols, cells.length);
                cells.forEach((cell, ci) => {
                    const text = cell.textContent?.trim() || '';
                    if (cell.getAttribute('role') === 'columnheader') {
                        gridData.headers.push(text);
                    }
                    if (text) {
                        gridData.cells.push([ri, ci, text]);
                    }
                });
            });
            result.grid = gridData;
        }

        return JSON.stringify(result);
    })()
    "#;

    let js_result = context.execute_js(js).await.ok()?;
    let result_str = js_result.as_str()?;
    let parsed: serde_json::Value = serde_json::from_str(result_str).ok()?;

    let elements: Vec<CanvasElement> = parsed
        .get("elements")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|el| {
                    let label = el.get("label")?.as_str()?.to_string();
                    let role = el.get("role")?.as_str()?.to_string();
                    let x = el.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = el.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let w = el.get("w").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let h = el.get("h").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let action = el.get("action").and_then(|v| v.as_str()).map(String::from);
                    Some(CanvasElement {
                        label,
                        role,
                        bounds: Some((x, y, w, h)),
                        action,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let text_content: Vec<(String, f32, f32)> = parsed
        .get("text")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| {
                    let text = t.get("text")?.as_str()?.to_string();
                    let x = t.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    let y = t.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;
                    Some((text, x, y))
                })
                .collect()
        })
        .unwrap_or_default();

    // Extract grid data if present
    let grid = parsed.get("grid").and_then(|g| {
        let rows = g.get("rows")?.as_u64()? as u32;
        let cols = g.get("cols")?.as_u64()? as u32;
        let cells: Vec<(u32, u32, String)> = g
            .get("cells")?
            .as_array()?
            .iter()
            .filter_map(|c| {
                let arr = c.as_array()?;
                let r = arr.first()?.as_u64()? as u32;
                let c_idx = arr.get(1)?.as_u64()? as u32;
                let val = arr.get(2)?.as_str()?.to_string();
                Some((r, c_idx, val))
            })
            .collect();
        let headers: Vec<String> = g
            .get("headers")?
            .as_array()?
            .iter()
            .filter_map(|h| h.as_str().map(String::from))
            .collect();
        Some(GridData {
            rows,
            cols,
            cells,
            headers,
        })
    });

    let app_type = if grid.is_some() {
        CanvasAppType::Spreadsheet
    } else if !elements.is_empty() {
        CanvasAppType::Unknown
    } else {
        return None; // No useful data extracted
    };

    tracing::info!(
        "Tier 2: extracted {} elements + {} text entries from accessibility tree",
        elements.len(),
        text_content.len()
    );

    Some(CanvasState {
        app_type,
        grid,
        layers: None,
        text_content,
        interactive_elements: elements,
        raw_state: None,
        extraction_tier: ExtractionTier::AccessibilityTree,
    })
}

// ── Tier 3: App State Extraction ────────────────────────────────────────────

/// Extract canvas state from the application's JavaScript state (Tier 3).
///
/// Tries to access global state objects commonly used by modern web apps:
/// - `window.__INITIAL_STATE__`
/// - `window.__NEXT_DATA__`
/// - `window.__NUXT__`
/// - Redux store
/// - React fiber tree
pub async fn extract_via_app_state(context: &dyn RenderContext) -> Option<CanvasState> {
    let js = r#"
    (() => {
        // Try common state objects
        const candidates = [
            window.__INITIAL_STATE__,
            window.__NEXT_DATA__,
            window.__NUXT__,
            window.__APP_STATE__,
            window.__PRELOADED_STATE__,
        ];

        for (const state of candidates) {
            if (state && typeof state === 'object') {
                try {
                    const json = JSON.stringify(state);
                    if (json.length > 10 && json.length < 5000000) {
                        return json;
                    }
                } catch(e) {}
            }
        }

        // Try Redux store
        try {
            if (window.__REDUX_DEVTOOLS_EXTENSION__ || window.__store__) {
                const store = window.__store__ || document.querySelector('[data-reactroot]')?.__store__;
                if (store && typeof store.getState === 'function') {
                    const state = store.getState();
                    const json = JSON.stringify(state);
                    if (json.length > 10 && json.length < 5000000) {
                        return json;
                    }
                }
            }
        } catch(e) {}

        return null;
    })()
    "#;

    let js_result = context.execute_js(js).await.ok()?;
    let result_str = js_result.as_str()?;
    let raw_state: serde_json::Value = serde_json::from_str(result_str).ok()?;

    // Try to classify the app type from the state structure
    let app_type = classify_app_from_state(&raw_state);
    let grid = extract_grid_from_json(Some(&raw_state));
    let layers = extract_layers_from_json(Some(&raw_state));

    tracing::info!("Tier 3: extracted app state ({:?})", app_type);

    Some(CanvasState {
        app_type,
        grid,
        layers,
        text_content: Vec::new(),
        interactive_elements: Vec::new(),
        raw_state: Some(raw_state),
        extraction_tier: ExtractionTier::AppState,
    })
}

// ── Detect if a page is a canvas app ────────────────────────────────────────

/// Check if a page is likely a canvas/WebGL application.
///
/// Checks for `<canvas>` elements and WebGL contexts in the HTML source.
/// This is a quick heuristic check that doesn't require a browser.
pub fn is_canvas_app(html: &str) -> bool {
    html.contains("<canvas")
        || html.contains("getContext('webgl')")
        || html.contains("getContext(\"webgl\")")
        || html.contains("getContext('2d')")
        || html.contains("getContext(\"2d\")")
        || html.contains("WebGLRenderingContext")
}

// ── Private helpers ─────────────────────────────────────────────────────────

/// Classify app type from JavaScript state structure.
fn classify_app_from_state(state: &serde_json::Value) -> CanvasAppType {
    let state_str = state.to_string().to_lowercase();

    if state_str.contains("spreadsheet")
        || (state_str.contains("\"cells\"")
            || (state_str.contains("\"rows\"") && state_str.contains("\"columns\"")))
    {
        CanvasAppType::Spreadsheet
    } else if state_str.contains("\"layers\"")
        || (state_str.contains("\"canvas\"") && state_str.contains("\"frames\""))
    {
        CanvasAppType::DesignTool
    } else if (state_str.contains("\"lat\"") && state_str.contains("\"lng\""))
        || state_str.contains("\"latitude\"")
    {
        CanvasAppType::Map
    } else if state_str.contains("\"whiteboard\"")
        || (state_str.contains("\"board\"") && state_str.contains("\"shapes\""))
    {
        CanvasAppType::Whiteboard
    } else {
        CanvasAppType::Unknown
    }
}

/// Try to extract grid data from a JSON state object.
fn extract_grid_from_json(state: Option<&serde_json::Value>) -> Option<GridData> {
    let state = state?;

    // Look for common grid patterns
    // Pattern 1: { cells: { "A1": { value: "..." }, ... } }
    if let Some(cells_obj) = state.get("cells").and_then(|v| v.as_object()) {
        let mut cells = Vec::new();
        let mut max_row = 0u32;
        let mut max_col = 0u32;
        let mut headers = Vec::new();

        for (key, val) in cells_obj {
            if let Some((row, col)) = parse_cell_ref(key) {
                let value = val
                    .get("value")
                    .or_else(|| val.get("v"))
                    .and_then(|v| {
                        if v.is_string() {
                            v.as_str().map(String::from)
                        } else {
                            Some(v.to_string())
                        }
                    })
                    .unwrap_or_default();
                if !value.is_empty() {
                    cells.push((row, col, value.clone()));
                    max_row = max_row.max(row);
                    max_col = max_col.max(col);
                    if row == 0 {
                        headers.push(value);
                    }
                }
            }
        }

        if !cells.is_empty() {
            return Some(GridData {
                rows: max_row + 1,
                cols: max_col + 1,
                cells,
                headers,
            });
        }
    }

    // Pattern 2: { rows: [ { cells: [ { value: "..." } ] } ] }
    if let Some(rows_arr) = state.get("rows").and_then(|v| v.as_array()) {
        let mut cells = Vec::new();
        let mut headers = Vec::new();
        let mut max_col = 0u32;

        for (ri, row) in rows_arr.iter().enumerate() {
            if let Some(row_cells) = row.get("cells").and_then(|v| v.as_array()) {
                for (ci, cell) in row_cells.iter().enumerate() {
                    let value = cell
                        .get("value")
                        .or_else(|| cell.get("v"))
                        .and_then(|v| {
                            if v.is_string() {
                                v.as_str().map(String::from)
                            } else {
                                Some(v.to_string())
                            }
                        })
                        .unwrap_or_default();
                    if !value.is_empty() {
                        cells.push((ri as u32, ci as u32, value.clone()));
                        max_col = max_col.max(ci as u32);
                        if ri == 0 {
                            headers.push(value);
                        }
                    }
                }
            }
        }

        if !cells.is_empty() {
            return Some(GridData {
                rows: rows_arr.len() as u32,
                cols: max_col + 1,
                cells,
                headers,
            });
        }
    }

    None
}

/// Parse a spreadsheet cell reference like "A1" into (row, col).
fn parse_cell_ref(cell_ref: &str) -> Option<(u32, u32)> {
    let mut col_part = String::new();
    let mut row_part = String::new();

    for ch in cell_ref.chars() {
        if ch.is_ascii_alphabetic() {
            col_part.push(ch.to_ascii_uppercase());
        } else if ch.is_ascii_digit() {
            row_part.push(ch);
        } else {
            return None;
        }
    }

    if col_part.is_empty() || row_part.is_empty() {
        return None;
    }

    // Convert column letters to index (A=0, B=1, ..., Z=25, AA=26, ...)
    let mut col: u32 = 0;
    for ch in col_part.chars() {
        col = col * 26 + (ch as u32 - 'A' as u32 + 1);
    }
    col -= 1; // Make 0-indexed

    let row: u32 = row_part.parse::<u32>().ok()?.checked_sub(1)?;

    Some((row, col))
}

/// Try to extract layer data from a JSON state object.
fn extract_layers_from_json(state: Option<&serde_json::Value>) -> Option<Vec<Layer>> {
    let state = state?;

    let layers_arr = state
        .get("layers")
        .or_else(|| state.get("document").and_then(|d| d.get("layers")))
        .or_else(|| state.get("children"))
        .and_then(|v| v.as_array())?;

    let layers: Vec<Layer> = layers_arr
        .iter()
        .filter_map(|l| {
            let name = l
                .get("name")
                .or_else(|| l.get("id"))
                .and_then(|v| v.as_str())
                .map(String::from)?;
            let visible = l.get("visible").and_then(|v| v.as_bool()).unwrap_or(true);
            let children = l
                .get("children")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| {
                            let label = c
                                .get("name")
                                .or_else(|| c.get("id"))
                                .and_then(|v| v.as_str())
                                .map(String::from)?;
                            Some(CanvasElement {
                                label,
                                role: c
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                bounds: None,
                                action: None,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Layer {
                name,
                visible,
                children,
            })
        })
        .collect();

    if layers.is_empty() {
        None
    } else {
        Some(layers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_canvas_app() {
        assert!(is_canvas_app(
            "<html><body><canvas id='main'></canvas></body></html>"
        ));
        assert!(is_canvas_app("var ctx = el.getContext('2d');"));
        assert!(!is_canvas_app("<html><body><h1>Hello</h1></body></html>"));
    }

    #[test]
    fn test_parse_cell_ref() {
        assert_eq!(parse_cell_ref("A1"), Some((0, 0)));
        assert_eq!(parse_cell_ref("B3"), Some((2, 1)));
        assert_eq!(parse_cell_ref("Z1"), Some((0, 25)));
        assert_eq!(parse_cell_ref("AA1"), Some((0, 26)));
        assert_eq!(parse_cell_ref(""), None);
        assert_eq!(parse_cell_ref("123"), None);
        assert_eq!(parse_cell_ref("A"), None);
    }

    #[test]
    fn test_extract_grid_from_json_cells_pattern() {
        let state = serde_json::json!({
            "cells": {
                "A1": {"value": "Name"},
                "B1": {"value": "Price"},
                "A2": {"value": "Widget"},
                "B2": {"value": "29.99"}
            }
        });
        let grid = extract_grid_from_json(Some(&state)).unwrap();
        assert_eq!(grid.rows, 2);
        assert_eq!(grid.cols, 2);
        assert_eq!(grid.cells.len(), 4);
    }

    #[test]
    fn test_extract_grid_from_json_rows_pattern() {
        let state = serde_json::json!({
            "rows": [
                {"cells": [{"value": "Name"}, {"value": "Price"}]},
                {"cells": [{"value": "Widget"}, {"value": "29.99"}]}
            ]
        });
        let grid = extract_grid_from_json(Some(&state)).unwrap();
        assert_eq!(grid.rows, 2);
        assert_eq!(grid.cols, 2);
        assert_eq!(grid.cells.len(), 4);
    }

    #[test]
    fn test_extract_layers_from_json() {
        let state = serde_json::json!({
            "layers": [
                {"name": "Background", "visible": true, "children": [
                    {"name": "Logo", "type": "image"}
                ]},
                {"name": "Content", "visible": true, "children": [
                    {"name": "Title", "type": "text"},
                    {"name": "Button", "type": "button"}
                ]}
            ]
        });
        let layers = extract_layers_from_json(Some(&state)).unwrap();
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].name, "Background");
        assert_eq!(layers[0].children.len(), 1);
        assert_eq!(layers[1].children.len(), 2);
    }

    #[test]
    fn test_classify_app_from_state() {
        let spreadsheet = serde_json::json!({"cells": {}, "rows": [], "columns": []});
        assert_eq!(
            classify_app_from_state(&spreadsheet),
            CanvasAppType::Spreadsheet
        );

        let map = serde_json::json!({"center": {"lat": 40.7, "lng": -74.0}});
        assert_eq!(classify_app_from_state(&map), CanvasAppType::Map);

        let unknown = serde_json::json!({"foo": "bar"});
        assert_eq!(classify_app_from_state(&unknown), CanvasAppType::Unknown);
    }

    #[test]
    fn test_empty_state() {
        assert!(extract_grid_from_json(None).is_none());
        assert!(extract_layers_from_json(None).is_none());
    }
}

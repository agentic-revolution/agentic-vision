//! ACT handler — execute actions on live pages.

use crate::map::types::OpCode;
use crate::renderer::RenderContext;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Result of executing an action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActResult {
    /// Whether the action succeeded.
    pub success: bool,
    /// The new URL after the action (if navigation occurred).
    pub new_url: Option<String>,
    /// Updated features after the action.
    pub features: Vec<(usize, f32)>,
}

/// Parameters for an action.
#[derive(Debug, Clone)]
pub struct ActRequest {
    /// The URL of the page to act on.
    pub url: String,
    /// The opcode to execute.
    pub opcode: OpCode,
    /// Additional parameters for the action.
    pub params: std::collections::HashMap<String, serde_json::Value>,
    /// Session ID for multi-step flows.
    pub session_id: Option<String>,
}

/// Execute an action on a live page.
pub async fn execute_action(
    context: &mut dyn RenderContext,
    url: &str,
    opcode: &OpCode,
    params: &std::collections::HashMap<String, serde_json::Value>,
) -> Result<ActResult> {
    // Navigate to the target page
    context.navigate(url, 30_000).await?;

    // Build JS to find and interact with the element
    let js = build_action_script(opcode, params);
    let result = context.execute_js(&js).await?;

    // Wait for any page updates
    let _ = context
        .execute_js("new Promise(r => setTimeout(r, 1000))")
        .await;

    // Get the new URL
    let new_url = context.get_url().await.ok();

    let success = result
        .as_object()
        .and_then(|o| o.get("success"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(ActResult {
        success,
        new_url,
        features: Vec::new(),
    })
}

/// Build a JavaScript snippet to execute the given opcode action.
fn build_action_script(
    opcode: &OpCode,
    params: &std::collections::HashMap<String, serde_json::Value>,
) -> String {
    match (opcode.category, opcode.action) {
        // Navigation: click
        (0x01, 0x00) => {
            let selector = params
                .get("selector")
                .and_then(|v| v.as_str())
                .unwrap_or("a");
            format!(
                r#"(() => {{
                    const el = document.querySelector('{}');
                    if (el) {{ el.click(); return {{ success: true }}; }}
                    return {{ success: false }};
                }})()"#,
                selector.replace('\'', "\\'")
            )
        }
        // Commerce: add to cart
        (0x02, 0x00) => {
            r#"(() => {
                const btn = document.querySelector('[data-action="add-to-cart"], button[name="add-to-cart"], .add-to-cart');
                if (btn) { btn.click(); return { success: true }; }
                const btns = [...document.querySelectorAll('button')].filter(b => /add to cart/i.test(b.textContent));
                if (btns.length) { btns[0].click(); return { success: true }; }
                return { success: false };
            })()"#.to_string()
        }
        // Form: submit
        (0x03, 0x05) => {
            let form_selector = params
                .get("form_selector")
                .and_then(|v| v.as_str())
                .unwrap_or("form");
            format!(
                r#"(() => {{
                    const form = document.querySelector('{}');
                    if (form) {{ form.submit(); return {{ success: true }}; }}
                    return {{ success: false }};
                }})()"#,
                form_selector.replace('\'', "\\'")
            )
        }
        // Auth: login click
        (0x04, 0x00) => {
            r#"(() => {
                const btn = document.querySelector('button[type="submit"], input[type="submit"], .login-btn');
                if (btn) { btn.click(); return { success: true }; }
                return { success: false };
            })()"#.to_string()
        }
        // Form: fill input
        (0x03, 0x00) => {
            let selector = params
                .get("selector")
                .and_then(|v| v.as_str())
                .unwrap_or("input");
            let value = params
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!(
                r#"(() => {{
                    const el = document.querySelector('{}');
                    if (el) {{
                        el.value = '{}';
                        el.dispatchEvent(new Event('input', {{ bubbles: true }}));
                        return {{ success: true }};
                    }}
                    return {{ success: false }};
                }})()"#,
                selector.replace('\'', "\\'"),
                value.replace('\'', "\\'")
            )
        }
        // Default: try to click based on label
        _ => {
            r#"(() => { return { success: false, reason: "unsupported opcode" }; })()"#
                .to_string()
        }
    }
}

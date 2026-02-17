//! Load and inject extraction scripts into browser contexts.

use crate::renderer::RenderContext;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Names of the extraction scripts.
const EXTRACTOR_NAMES: &[&str] = &["content", "actions", "navigation", "structure", "metadata"];

/// Combined result from all extractors.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub content: serde_json::Value,
    pub actions: serde_json::Value,
    pub navigation: serde_json::Value,
    pub structure: serde_json::Value,
    pub metadata: serde_json::Value,
}

/// Loads extraction scripts from disk or embedded paths.
pub struct ExtractionLoader {
    scripts: Vec<(String, String)>,
}

impl ExtractionLoader {
    /// Create a new loader, reading scripts from the dist directory.
    pub fn new() -> Result<Self> {
        let mut scripts = Vec::new();

        // Look for scripts in several locations
        let search_paths = vec![
            // Relative to binary
            PathBuf::from("extractors/dist"),
            // Relative to workspace root
            PathBuf::from("../extractors/dist"),
            // Embedded in source tree
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("src/extraction/scripts"),
        ];

        for name in EXTRACTOR_NAMES {
            let filename = format!("{name}.js");
            let mut found = false;

            for base in &search_paths {
                let path = base.join(&filename);
                if path.exists() {
                    let content = std::fs::read_to_string(&path)
                        .with_context(|| format!("reading {}", path.display()))?;
                    scripts.push((name.to_string(), content));
                    found = true;
                    break;
                }
            }

            if !found {
                // Use a minimal fallback script that returns empty data
                scripts.push((
                    name.to_string(),
                    format!(
                        "var CortexExtractor_{name} = {{ default: function() {{ return []; }} }};"
                    ),
                ));
            }
        }

        Ok(Self { scripts })
    }

    /// Inject all extraction scripts into a page and collect results.
    pub async fn inject_and_run(
        &self,
        context: &dyn RenderContext,
    ) -> Result<ExtractionResult> {
        // Inject all scripts first
        for (name, script) in &self.scripts {
            context
                .execute_js(script)
                .await
                .with_context(|| format!("injecting {name} extractor"))?;
        }

        // Run the extraction entry point
        let run_script = r#"
            (function() {
                var result = {};
                try {
                    if (typeof __cortex_extractContent === 'function')
                        result.content = __cortex_extractContent(document);
                    else if (typeof CortexExtractor_content !== 'undefined')
                        result.content = [];
                    else
                        result.content = [];
                } catch(e) { result.content = []; }

                try {
                    if (typeof __cortex_extractActions === 'function')
                        result.actions = __cortex_extractActions(document);
                    else
                        result.actions = [];
                } catch(e) { result.actions = []; }

                try {
                    if (typeof __cortex_extractNavigation === 'function')
                        result.navigation = __cortex_extractNavigation(document);
                    else
                        result.navigation = [];
                } catch(e) { result.navigation = []; }

                try {
                    if (typeof __cortex_extractStructure === 'function')
                        result.structure = __cortex_extractStructure(document);
                    else
                        result.structure = {};
                } catch(e) { result.structure = {}; }

                try {
                    if (typeof __cortex_extractMetadata === 'function')
                        result.metadata = __cortex_extractMetadata(document);
                    else
                        result.metadata = {};
                } catch(e) { result.metadata = {}; }

                return result;
            })()
        "#;

        let result = context
            .execute_js(run_script)
            .await
            .context("running extraction")?;

        Ok(ExtractionResult {
            content: result.get("content").cloned().unwrap_or_default(),
            actions: result.get("actions").cloned().unwrap_or_default(),
            navigation: result.get("navigation").cloned().unwrap_or_default(),
            structure: result.get("structure").cloned().unwrap_or_default(),
            metadata: result.get("metadata").cloned().unwrap_or_default(),
        })
    }
}

//! PERCEIVE handler — render a single URL and return its encoding.

use crate::cartography::feature_encoder;
use crate::cartography::page_classifier;
use crate::extraction::loader::{ExtractionLoader, ExtractionResult};
use crate::renderer::{NavigationResult, RenderContext};
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Result of perceiving a single page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceiveResult {
    /// The original URL that was requested.
    pub url: String,
    /// The final URL after redirects.
    pub final_url: String,
    /// Classified page type.
    pub page_type: u8,
    /// Classification confidence.
    pub confidence: f32,
    /// 128-dimension feature vector (sparse: only non-zero entries).
    pub features: Vec<(usize, f32)>,
    /// Optional raw text content of the page.
    pub content: Option<String>,
    /// Load time in milliseconds.
    pub load_time_ms: u64,
}

/// Perceive a single URL: render, extract, encode.
pub async fn perceive(
    context: &mut dyn RenderContext,
    url: &str,
    include_content: bool,
) -> Result<PerceiveResult> {
    // Navigate to the page
    let nav_result = context.navigate(url, 30_000).await?;

    // Run extraction scripts
    let extraction = run_extraction(context).await.unwrap_or_default();

    // Classify the page
    let (page_type, confidence) =
        page_classifier::classify_page(&extraction, &nav_result.final_url);

    // Encode features
    let features = feature_encoder::encode_features(
        &extraction,
        &nav_result,
        &nav_result.final_url,
        page_type,
        confidence,
    );

    // Convert to sparse representation
    let sparse_features: Vec<(usize, f32)> = features
        .iter()
        .enumerate()
        .filter(|(_, &v)| v != 0.0)
        .map(|(i, &v)| (i, v))
        .collect();

    // Optionally extract text content
    let content = if include_content {
        extract_text_content(context).await.ok()
    } else {
        None
    };

    Ok(PerceiveResult {
        url: url.to_string(),
        final_url: nav_result.final_url,
        page_type: page_type as u8,
        confidence,
        features: sparse_features,
        content,
        load_time_ms: nav_result.load_time_ms,
    })
}

/// Run extraction scripts on the current page context.
async fn run_extraction(context: &dyn RenderContext) -> Result<ExtractionResult> {
    let loader = ExtractionLoader::new()?;
    loader.inject_and_run(context).await
}

/// Extract visible text content from the page.
async fn extract_text_content(context: &dyn RenderContext) -> Result<String> {
    let result = context
        .execute_js("document.body ? document.body.innerText : ''")
        .await?;
    Ok(result.as_str().unwrap_or("").to_string())
}

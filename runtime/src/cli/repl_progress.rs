// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

//! Animated progress display for the interactive REPL mapping operation.
//!
//! Uses `indicatif` to show layered acquisition progress with spinners
//! and completion markers.

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

/// Acquisition layer names and their display labels.
const LAYERS: &[&str] = &[
    "Sitemap discovery",
    "HTTP extraction",
    "Pattern engine",
    "API discovery",
    "Browser fallback",
];

/// Create a multi-step progress display for a mapping operation.
///
/// Returns the `MultiProgress` handle and a vector of individual progress bars.
pub fn create_mapping_progress() -> (MultiProgress, Vec<ProgressBar>) {
    let mp = MultiProgress::new();
    let mut bars = Vec::with_capacity(LAYERS.len());

    let spinner_style = ProgressStyle::with_template("  {spinner:.cyan} {msg}")
        .unwrap()
        .tick_chars("\u{25b8}\u{25b9}\u{25b8}\u{25b9}\u{25b8}");

    for &layer in LAYERS {
        let bar = mp.add(ProgressBar::new_spinner());
        bar.set_style(spinner_style.clone());
        bar.set_message(format!("{layer:<22} \x1b[2mwaiting\x1b[0m"));
        bar.enable_steady_tick(Duration::from_millis(120));
        bars.push(bar);
    }

    (mp, bars)
}

/// Mark a layer as active (currently running).
pub fn set_layer_active(bar: &ProgressBar, layer_name: &str, detail: &str) {
    let active_style = ProgressStyle::with_template("  {spinner:.blue} {msg}")
        .unwrap()
        .tick_chars("\u{2588}\u{2589}\u{258a}\u{258b}\u{258c}\u{258d}\u{258e}\u{258f} ");
    bar.set_style(active_style);
    bar.set_message(format!("{layer_name:<22} \x1b[34m{detail}\x1b[0m"));
}

/// Mark a layer as complete.
pub fn set_layer_done(bar: &ProgressBar, layer_name: &str, detail: &str) {
    bar.set_style(ProgressStyle::with_template("  {msg}").unwrap());
    bar.set_message(format!(
        "\x1b[32m\u{2713}\x1b[0m {layer_name:<22} \x1b[32m{detail}\x1b[0m"
    ));
    bar.finish();
}

/// Mark a layer as skipped.
pub fn set_layer_skipped(bar: &ProgressBar, layer_name: &str, reason: &str) {
    bar.set_style(ProgressStyle::with_template("  {msg}").unwrap());
    bar.set_message(format!(
        "\x1b[2m\u{25cb}\x1b[0m {layer_name:<22} \x1b[2m{reason}\x1b[0m"
    ));
    bar.finish();
}

/// Create a simple spinner for general operations.
pub fn create_spinner(message: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("  {spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("\u{25b8}\u{25b9}\u{25b8}\u{25b9}\u{25b8}"),
    );
    bar.set_message(message.to_string());
    bar.enable_steady_tick(Duration::from_millis(120));
    bar
}

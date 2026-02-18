#!/usr/bin/env node
// Copyright 2026 Cortex Contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Thin wrapper that delegates to the platform-native Cortex binary.
 */

const { execFileSync } = require("child_process");
const path = require("path");

const binary = path.join(__dirname, "cortex");

try {
  execFileSync(binary, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  process.exit(err.status ?? 1);
}

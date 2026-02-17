#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
echo "Building extraction scripts..."
mkdir -p dist
for extractor in core/*.ts; do
  [ -f "$extractor" ] || continue
  name=$(basename "$extractor" .ts)
  echo "  Building $name..."
  npx esbuild "$extractor" --bundle --format=iife --global-name="CortexExtractor_${name}" --outfile="dist/${name}.js" --platform=browser --target=es2020 2>/dev/null || echo "  Warning: $name build skipped (not implemented yet)"
done
echo "Done."

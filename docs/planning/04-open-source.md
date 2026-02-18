# 04 — Open Source Strategy

## License

Apache-2.0 for every file. No exceptions. No dual licensing. No CLA.

## Governance

Phase 1 (0-100 contributors): Benevolent Dictator. Project lead makes all final decisions.
Phase 2 (100+ contributors): Steering committee of 3-5 regular contributors.
Phase 3 (if adoption warrants): Consider joining CNCF or Linux Foundation.

## Community Infrastructure

- Code: GitHub
- Chat: Discord (#general, #help, #extractors, #development, #showcase)
- Docs: GitHub Pages from docs/
- CI: GitHub Actions
- Releases: GitHub Releases + PyPI + npm + crates.io
- Board: GitHub Projects
- Security: SECURITY.md + GitHub private vulnerability reporting

## Contribution Entry Points

Easiest: Add platform patterns to `runtime/src/acquisition/css_selectors.json` or `runtime/src/acquisition/platform_actions.json`. Write a custom browser extractor for a site category (see `docs/guides/writing-extractors.md`).
Medium: Improve URL pattern classifiers for specific domains. Add new platform detection to the acquisition engine.
Advanced: Core runtime improvements, new thin client languages, pathfinding optimizations.

## No Telemetry

Cortex never phones home. No analytics. No tracking. No usage data. No license checks. Everything local unless user explicitly configures cloud acceleration. This is non-negotiable.

## README

The README.md at repo root must contain:
1. One-line description: "Give any AI agent a complete map of any website in seconds."
2. Quick start (install + first map in 60 seconds)
3. How it works (3-sentence explanation)
4. Speed comparison table
5. Works with: framework list
6. Links to docs
7. Contributing link
8. License badge

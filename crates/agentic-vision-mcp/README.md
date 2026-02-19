# AgenticVision-MCP

**MCP server for AgenticVision — universal LLM access to persistent visual memory.**

[![crates.io](https://img.shields.io/crates/v/agentic-vision-mcp.svg)](https://crates.io/crates/agentic-vision-mcp)
[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)

## What it does

AgenticVision-MCP exposes the [AgenticVision](https://crates.io/crates/agentic-vision) engine over the [Model Context Protocol](https://modelcontextprotocol.io) (JSON-RPC 2.0 over stdio). Any MCP-compatible LLM gains persistent visual memory — capture screenshots, embed with CLIP ViT-B/32, compare, recall.

## Install

```bash
cargo install agentic-vision-mcp
```

## Configure Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "vision": {
      "command": "agentic-vision-mcp",
      "args": ["--vision", "~/.vision.avis", "serve"]
    }
  }
}
```

## Configure VS Code / Cursor

Add to `.vscode/settings.json`:

```json
{
  "mcp.servers": {
    "vision": {
      "command": "agentic-vision-mcp",
      "args": ["--vision", "${workspaceFolder}/.vision/project.avis", "serve"]
    }
  }
}
```

> **Do not use `/tmp` for vision files** — macOS and Linux clear this directory periodically. Use `~/.vision.avis` for persistent storage.

## MCP Surface Area

| Category | Count | Examples |
|:---|---:|:---|
| **Tools** | 10 | `vision_capture`, `vision_similar`, `vision_diff`, `vision_compare`, `vision_query`, `vision_ocr`, `vision_track`, `vision_link`, `session_start`, `session_end` |
| **Resources** | 6 | `avis://capture/{id}`, `avis://session/{id}`, `avis://timeline/{start}/{end}`, `avis://similar/{id}`, `avis://stats`, `avis://recent` |
| **Prompts** | 4 | `observe`, `compare`, `track`, `describe` |

## How it works

1. **Capture** — `vision_capture` accepts images from files, base64, or URLs. Embeds with CLIP ViT-B/32, stores in `.avis` binary format.
2. **Query** — `vision_query` retrieves by time, description, or recency. `vision_similar` finds visually similar captures by cosine similarity.
3. **Compare** — `vision_compare` for side-by-side LLM analysis. `vision_diff` for pixel-level differencing with 8×8 grid region detection.
4. **Link** — `vision_link` connects captures to [AgenticMemory](https://github.com/xeo-labs/agentic-memory) cognitive graph nodes.

## Performance

| Operation | Time |
|:---|---:|
| MCP tool round-trip | **7.2 ms** |
| Image capture | **47 ms** |
| Similarity search (top-5) | **1-2 ms** |
| Visual diff | **<1 ms** |

## Links

- [GitHub](https://github.com/xeo-labs/agentic-vision)
- [Core Library](https://crates.io/crates/agentic-vision)
- [AgenticMemory](https://github.com/xeo-labs/agentic-memory) — Persistent cognitive memory for AI agents

## License

MIT

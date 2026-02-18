# Quickstart: From Zero to First Map in 60 Seconds

## Install

```bash
# Install Cortex
cargo install cortex-runtime
```

That's it. No other setup needed.

## Map Your First Site

```bash
cortex map example.com
```

On first run, Cortex will automatically:
1. Start the background daemon
2. Map the site via HTTP-first layered acquisition (no browser needed)
3. Download Chromium for Testing (~130 MB) only if browser fallback is needed

Most sites are mapped entirely via HTTP. Chromium is downloaded on-demand for sites with very low structured data coverage or when using ACT operations.

Output:
```
Mapping example.com...
Map complete:
  Domain:   example.com
  Nodes:    15
  Edges:    22
  Rendered: 10
  Duration: 4.32s
```

## Query the Map

```bash
cortex query example.com --type article --limit 5
```

## Use from Python

```python
import cortex_client

# Map a site (auto-starts daemon if needed)
site = cortex_client.map("example.com")

# Find product pages
products = site.filter(page_type=4, limit=10)
for p in products:
    print(f"{p.url} (confidence: {p.confidence:.2f})")

# Find path from home to checkout
path = site.pathfind(0, target_node)
print(f"Path: {path.hops} hops, {len(path.required_actions)} actions needed")
```

## Use from TypeScript

```typescript
import { map } from '@cortex-ai/client';

const site = await map('example.com');
const results = await site.filter({ pageType: 4, limit: 10 });
results.forEach(r => console.log(r.url));
```

## Other Useful Commands

```bash
# Check environment
cortex doctor

# Search by page type and features
cortex query example.com --type product_detail --price-lt 100

# Find shortest path between nodes
cortex pathfind example.com --from 0 --to 42

# Machine-readable output
cortex map example.com --json

# Clear cached maps
cortex cache clear

# Stop the daemon
cortex stop
```

## Shell Completions

```bash
# Bash
cortex completions bash >> ~/.bashrc

# Zsh
cortex completions zsh >> ~/.zshrc

# Fish
cortex completions fish > ~/.config/fish/completions/cortex.fish
```

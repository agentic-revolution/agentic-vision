# Quickstart: From Zero to First Map in 60 Seconds

## Install

```bash
# Install Cortex
cargo install cortex-runtime

# Install Chromium for Testing
cortex install

# Verify everything works
cortex doctor
```

## Start the Daemon

```bash
cortex start
```

## Map Your First Site

```bash
cortex map example.com --max-render 10
```

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

## Stop the Daemon

```bash
cortex stop
```

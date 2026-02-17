# cortex-client

Thin Python client for [Cortex](https://github.com/cortex-ai/cortex) — the rapid web cartographer for AI agents.

```python
from cortex_client import map
site = map("amazon.com")
products = site.filter(page_type=0x04, features={48: {"lt": 300}})
```

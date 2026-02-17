# Interactive Sessions: Multi-Step Flows with ACT

Cortex supports persistent browser sessions for multi-step workflows
like login, add-to-cart, and checkout.

## How Sessions Work

A session holds a persistent browser context with cookies and state.
Actions executed within a session share the same browser context.

## Python Example: Login Flow

```python
import cortex_client

# Map the site first
site = cortex_client.map("shop.example.com")

# Find the login page
login_pages = site.filter(page_type=8)  # PageType::Login = 0x08
login_node = login_pages[0].index

# Start a session and log in
result = site.act(
    node=login_node,
    opcode=(0x03, 0x00),  # Form: fill input
    params={"selector": "#email", "value": "user@example.com"},
    session_id="my-session",
)

# Submit the login form
result = site.act(
    node=login_node,
    opcode=(0x04, 0x00),  # Auth: login
    session_id="my-session",
)

# Navigate to products (still logged in)
products = site.filter(page_type=4)
```

## OpCode Reference

| Category | Action | OpCode | Description |
|----------|--------|--------|-------------|
| Navigation | Click | (0x01, 0x00) | Click a link |
| Commerce | Add to Cart | (0x02, 0x00) | Add item to cart |
| Form | Fill Input | (0x03, 0x00) | Fill a form field |
| Form | Submit | (0x03, 0x05) | Submit a form |
| Auth | Login | (0x04, 0x00) | Click login button |

## Session Timeout

Sessions expire after 1 hour of inactivity by default.

# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""AutoGen function-call tools for Cortex web cartography.

Register these with an AutoGen ``AssistantAgent`` via
``register_for_llm`` / ``register_for_execution``, or add them to
``llm_config["functions"]`` for older AutoGen versions.

Usage::

    from cortex_autogen import cortex_map, cortex_query, cortex_act
    from autogen import AssistantAgent, UserProxyAgent

    assistant = AssistantAgent("assistant", llm_config=llm_config)
    user = UserProxyAgent("user", code_execution_config=False)

    assistant.register_for_llm(name="cortex_map")(cortex_map)
    user.register_for_execution(name="cortex_map")(cortex_map)
"""
from __future__ import annotations

import json
from typing import Any, Optional

import cortex_client


def cortex_map(domain: str, max_render: int = 50) -> str:
    """Map an entire website into a navigable graph.

    Args:
        domain: Domain to map (e.g. ``'amazon.com'``).
        max_render: Maximum pages to render with a browser.

    Returns:
        JSON summary of the mapped site.
    """
    sm = cortex_client.map(domain, max_render=max_render)
    return json.dumps(
        {
            "domain": sm.domain,
            "node_count": sm.node_count,
            "edge_count": sm.edge_count,
        }
    )


def cortex_query(
    domain: str,
    page_type: Optional[int] = None,
    limit: int = 20,
) -> str:
    """Search a mapped site for pages matching criteria.

    Args:
        domain: Domain to query (must be previously mapped).
        page_type: Filter by page type code.
        limit: Maximum results.

    Returns:
        JSON array of matching pages.
    """
    sm = cortex_client.map(domain, max_render=5)
    results = sm.filter(page_type=page_type, limit=limit)
    return json.dumps(
        [{"index": m.index, "url": m.url, "page_type": m.page_type} for m in results]
    )


def cortex_act(
    domain: str,
    node: int,
    opcode: list[int],
    params: Optional[dict[str, Any]] = None,
) -> str:
    """Execute an action on a live webpage.

    Args:
        domain: Domain to act on.
        node: Target node index.
        opcode: Action opcode as ``[category, action]``.
        params: Action-specific parameters.

    Returns:
        JSON result of the action.
    """
    sm = cortex_client.map(domain, max_render=5)
    result = sm.act(node, tuple(opcode), **(params or {}))
    return json.dumps({"success": result.success, "new_url": result.new_url})

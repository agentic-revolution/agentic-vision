# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Semantic Kernel plugin exposing Cortex tools as kernel functions.

Usage::

    from semantic_kernel import Kernel
    from cortex_semantic_kernel import CortexPlugin

    kernel = Kernel()
    kernel.add_plugin(CortexPlugin(), plugin_name="cortex")
"""
from __future__ import annotations

import json
from typing import Annotated, Any, Optional

try:
    from semantic_kernel.functions import kernel_function
except ImportError:  # pragma: no cover
    # Fallback: make the decorator a no-op so the module can still be imported
    # during packaging or environments without semantic-kernel installed.
    def kernel_function(  # type: ignore[misc]
        description: str = "",
        name: str | None = None,
    ) -> Any:
        def _dec(fn: Any) -> Any:
            return fn

        return _dec


import cortex_client


class CortexPlugin:
    """Cortex web cartography plugin for Semantic Kernel."""

    @kernel_function(description="Map an entire website into a navigable graph.", name="cortex_map")
    def map_site(
        self,
        domain: Annotated[str, "Domain to map (e.g. 'amazon.com')"],
        max_render: Annotated[int, "Max pages to render with browser"] = 50,
    ) -> Annotated[str, "JSON summary of the mapped site"]:
        sm = cortex_client.map(domain, max_render=max_render)
        return json.dumps(
            {
                "domain": sm.domain,
                "node_count": sm.node_count,
                "edge_count": sm.edge_count,
            }
        )

    @kernel_function(
        description="Search a mapped site for pages matching criteria.",
        name="cortex_query",
    )
    def query_site(
        self,
        domain: Annotated[str, "Domain to query (must be previously mapped)"],
        page_type: Annotated[Optional[int], "Page type code filter"] = None,
        limit: Annotated[int, "Maximum results"] = 20,
    ) -> Annotated[str, "JSON array of matching pages"]:
        sm = cortex_client.map(domain, max_render=5)
        results = sm.filter(page_type=page_type, limit=limit)
        return json.dumps(
            [{"index": m.index, "url": m.url, "page_type": m.page_type} for m in results]
        )

    @kernel_function(
        description="Find shortest path between two pages on a mapped site.",
        name="cortex_pathfind",
    )
    def pathfind(
        self,
        domain: Annotated[str, "Domain to pathfind in"],
        from_node: Annotated[int, "Source node index"],
        to_node: Annotated[int, "Target node index"],
    ) -> Annotated[str, "JSON path result"]:
        sm = cortex_client.map(domain, max_render=5)
        path = sm.pathfind(from_node, to_node)
        if path is None:
            return json.dumps({"path": None})
        return json.dumps(
            {"nodes": path.nodes, "hops": path.hops, "total_weight": path.total_weight}
        )

    @kernel_function(
        description="Execute an action on a live webpage.",
        name="cortex_act",
    )
    def act(
        self,
        domain: Annotated[str, "Domain to act on"],
        node: Annotated[int, "Target node index"],
        opcode: Annotated[str, "Action opcode as JSON array [category, action]"],
        params: Annotated[str, "Action params as JSON object"] = "{}",
    ) -> Annotated[str, "JSON result of the action"]:
        op = json.loads(opcode)
        p = json.loads(params)
        sm = cortex_client.map(domain, max_render=5)
        result = sm.act(node, tuple(op), **p)
        return json.dumps({"success": result.success, "new_url": result.new_url})

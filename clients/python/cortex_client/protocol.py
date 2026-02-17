# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Protocol message builders for the Cortex socket protocol."""

from __future__ import annotations

from typing import Any


def map_request(
    domain: str,
    *,
    max_nodes: int = 50000,
    max_render: int = 200,
    max_time_ms: int = 10000,
    respect_robots: bool = True,
) -> dict[str, Any]:
    """Build a MAP request."""
    return {
        "domain": domain,
        "max_nodes": max_nodes,
        "max_render": max_render,
        "max_time_ms": max_time_ms,
        "respect_robots": respect_robots,
    }


def query_request(
    domain: str,
    *,
    page_type: int | list[int] | None = None,
    features: dict[int, dict[str, float]] | None = None,
    flags: dict[str, bool] | None = None,
    sort_by: tuple[int, str] | None = None,
    limit: int = 100,
) -> dict[str, Any]:
    """Build a QUERY request."""
    params: dict[str, Any] = {"domain": domain, "limit": limit}
    if page_type is not None:
        params["page_type"] = page_type if isinstance(page_type, list) else [page_type]
    if features:
        params["features"] = features
    if flags:
        params["flags"] = flags
    if sort_by:
        params["sort_by"] = {"dimension": sort_by[0], "direction": sort_by[1]}
    return params


def pathfind_request(
    domain: str,
    from_node: int,
    to_node: int,
    *,
    avoid_flags: list[str] | None = None,
    minimize: str = "hops",
) -> dict[str, Any]:
    """Build a PATHFIND request."""
    params: dict[str, Any] = {
        "domain": domain,
        "from": from_node,
        "to": to_node,
        "minimize": minimize,
    }
    if avoid_flags:
        params["avoid_flags"] = avoid_flags
    return params


def refresh_request(
    domain: str,
    *,
    nodes: list[int] | None = None,
    cluster: int | None = None,
    stale_threshold: float | None = None,
) -> dict[str, Any]:
    """Build a REFRESH request."""
    params: dict[str, Any] = {"domain": domain}
    if nodes is not None:
        params["nodes"] = nodes
    if cluster is not None:
        params["cluster"] = cluster
    if stale_threshold is not None:
        params["stale_threshold"] = stale_threshold
    return params


def act_request(
    domain: str,
    node: int,
    opcode: tuple[int, int],
    *,
    params: dict[str, Any] | None = None,
    session_id: str | None = None,
) -> dict[str, Any]:
    """Build an ACT request."""
    result: dict[str, Any] = {
        "domain": domain,
        "node": node,
        "opcode": list(opcode),
    }
    if params:
        result["params"] = params
    if session_id:
        result["session_id"] = session_id
    return result


def perceive_request(
    url: str,
    *,
    include_content: bool = True,
) -> dict[str, Any]:
    """Build a PERCEIVE request."""
    return {
        "url": url,
        "include_content": include_content,
    }


def watch_request(
    domain: str,
    *,
    nodes: list[int] | None = None,
    cluster: int | None = None,
    features: list[int] | None = None,
    interval_ms: int = 60000,
) -> dict[str, Any]:
    """Build a WATCH request."""
    params: dict[str, Any] = {"domain": domain, "interval_ms": interval_ms}
    if nodes is not None:
        params["nodes"] = nodes
    if cluster is not None:
        params["cluster"] = cluster
    if features is not None:
        params["features"] = features
    return params

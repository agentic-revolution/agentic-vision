# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Cortex Client — Thin client for the Cortex web cartography runtime."""

from __future__ import annotations

from dataclasses import dataclass, field

from .autostart import ensure_running
from .connection import Connection, DEFAULT_SOCKET_PATH
from .errors import (
    CortexConnectionError,
    CortexError,
    CortexMapError,
    CortexPathError,
    CortexResourceError,
    CortexTimeoutError,
)
from .sitemap import (
    ActResult,
    NodeMatch,
    Path,
    PathAction,
    RefreshResult,
    SiteMap,
    WatchDelta,
)
from . import protocol

__version__ = "0.1.0"

__all__ = [
    # Top-level functions
    "map",
    "map_many",
    "perceive",
    "perceive_many",
    "status",
    # Classes
    "SiteMap",
    "NodeMatch",
    "Path",
    "PathAction",
    "RefreshResult",
    "ActResult",
    "WatchDelta",
    "RuntimeStatus",
    "PageResult",
    "Connection",
    # Errors
    "CortexError",
    "CortexConnectionError",
    "CortexTimeoutError",
    "CortexResourceError",
    "CortexMapError",
    "CortexPathError",
]


@dataclass
class RuntimeStatus:
    """Status of the Cortex runtime."""

    version: str
    uptime_seconds: float
    active_contexts: int
    cached_maps: int
    memory_mb: float


@dataclass
class PageResult:
    """Result of perceiving a single page."""

    url: str
    final_url: str
    page_type: int
    confidence: float
    features: dict[int, float] = field(default_factory=dict)
    content: str | None = None


def map(
    domain: str,
    *,
    max_nodes: int = 50000,
    max_render: int = 200,
    max_time_ms: int = 10000,
    respect_robots: bool = True,
    socket_path: str = DEFAULT_SOCKET_PATH,
) -> SiteMap:
    """Map a website and return a navigable SiteMap.

    Args:
        domain: The domain to map (e.g. "example.com").
        max_nodes: Maximum number of nodes to include.
        max_render: Maximum pages to render with a browser.
        max_time_ms: Maximum mapping time in milliseconds.
        respect_robots: Whether to respect robots.txt.
        socket_path: Path to the Cortex Unix socket.

    Returns:
        A SiteMap object for querying and navigating.
    """
    ensure_running(socket_path)
    conn = Connection(socket_path)
    params = protocol.map_request(
        domain,
        max_nodes=max_nodes,
        max_render=max_render,
        max_time_ms=max_time_ms,
        respect_robots=respect_robots,
    )
    resp = conn.send("map", params)
    if "error" in resp:
        raise CortexMapError(resp["error"].get("message", "map failed"))
    result = resp.get("result", {})
    return SiteMap(
        conn=conn,
        domain=domain,
        node_count=result.get("node_count", 0),
        edge_count=result.get("edge_count", 0),
        map_path=result.get("map_path"),
    )


def map_many(
    domains: list[str],
    *,
    max_nodes: int = 50000,
    max_render: int = 200,
    max_time_ms: int = 10000,
    respect_robots: bool = True,
    socket_path: str = DEFAULT_SOCKET_PATH,
) -> list[SiteMap]:
    """Map multiple websites.

    Args:
        domains: List of domains to map.
        max_nodes: Maximum nodes per domain.
        max_render: Maximum pages to render per domain.
        max_time_ms: Maximum mapping time per domain.
        respect_robots: Whether to respect robots.txt.
        socket_path: Path to the Cortex Unix socket.

    Returns:
        List of SiteMap objects.
    """
    return [
        map(
            d,
            max_nodes=max_nodes,
            max_render=max_render,
            max_time_ms=max_time_ms,
            respect_robots=respect_robots,
            socket_path=socket_path,
        )
        for d in domains
    ]


def perceive(
    url: str,
    *,
    include_content: bool = True,
    socket_path: str = DEFAULT_SOCKET_PATH,
) -> PageResult:
    """Perceive a single page and return its encoding.

    Args:
        url: The URL to perceive.
        include_content: Whether to include raw text content.
        socket_path: Path to the Cortex Unix socket.

    Returns:
        A PageResult with the page's encoding and optional content.
    """
    ensure_running(socket_path)
    conn = Connection(socket_path)
    params = protocol.perceive_request(url, include_content=include_content)
    resp = conn.send("perceive", params)
    if "error" in resp:
        raise CortexResourceError(resp["error"].get("message", "perceive failed"))
    result = resp.get("result", {})
    return PageResult(
        url=url,
        final_url=result.get("final_url", url),
        page_type=result.get("page_type", 0),
        confidence=result.get("confidence", 0.0),
        features=result.get("features", {}),
        content=result.get("content"),
    )


def perceive_many(
    urls: list[str],
    *,
    include_content: bool = True,
    socket_path: str = DEFAULT_SOCKET_PATH,
) -> list[PageResult]:
    """Perceive multiple pages.

    Args:
        urls: List of URLs to perceive.
        include_content: Whether to include raw text content.
        socket_path: Path to the Cortex Unix socket.

    Returns:
        List of PageResult objects.
    """
    return [
        perceive(u, include_content=include_content, socket_path=socket_path)
        for u in urls
    ]


def status(
    *,
    socket_path: str = DEFAULT_SOCKET_PATH,
) -> RuntimeStatus:
    """Get Cortex runtime status.

    Args:
        socket_path: Path to the Cortex Unix socket.

    Returns:
        RuntimeStatus with version, uptime, and resource info.
    """
    ensure_running(socket_path)
    conn = Connection(socket_path)
    resp = conn.send("status")
    if "error" in resp:
        raise CortexConnectionError(resp["error"].get("message", "status failed"))
    result = resp.get("result", {})
    return RuntimeStatus(
        version=result.get("version", "unknown"),
        uptime_seconds=result.get("uptime_seconds", 0.0),
        active_contexts=result.get("active_contexts", 0),
        cached_maps=result.get("cached_maps", 0),
        memory_mb=result.get("memory_mb", 0.0),
    )

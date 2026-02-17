# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""SiteMap class for navigating mapped websites."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Iterator

from .connection import Connection
from .errors import CortexPathError, CortexResourceError
from . import protocol


@dataclass
class NodeMatch:
    """A matched node from a query."""

    index: int
    url: str
    page_type: int
    confidence: float
    features: dict[int, float] = field(default_factory=dict)
    similarity: float | None = None


@dataclass
class Path:
    """A path through the site graph."""

    nodes: list[int]
    total_weight: float
    hops: int
    required_actions: list[PathAction]


@dataclass
class PathAction:
    """An action required at a specific node along a path."""

    at_node: int
    opcode: tuple[int, int]


@dataclass
class RefreshResult:
    """Result of refreshing nodes."""

    updated_count: int
    changed_nodes: list[int]


@dataclass
class ActResult:
    """Result of executing an action."""

    success: bool
    new_url: str | None = None
    features: dict[int, float] = field(default_factory=dict)


@dataclass
class WatchDelta:
    """A change detected during watching."""

    node: int
    changed_features: dict[int, tuple[float, float]]
    timestamp: float


class SiteMap:
    """Navigable binary site map.

    Wraps protocol responses to provide a convenient query interface.
    """

    def __init__(
        self,
        conn: Connection,
        domain: str,
        node_count: int,
        edge_count: int,
        map_path: str | None = None,
    ) -> None:
        self._conn = conn
        self.domain = domain
        self.node_count = node_count
        self.edge_count = edge_count
        self.map_path = map_path

    def filter(
        self,
        *,
        page_type: int | list[int] | None = None,
        features: dict[int, dict[str, float]] | None = None,
        flags: dict[str, bool] | None = None,
        sort_by: tuple[int, str] | None = None,
        limit: int = 100,
    ) -> list[NodeMatch]:
        """Filter nodes by type, features, and flags."""
        params = protocol.query_request(
            self.domain,
            page_type=page_type,
            features=features,
            flags=flags,
            sort_by=sort_by,
            limit=limit,
        )
        resp = self._conn.send("query", params)
        return _parse_node_matches(resp)

    def nearest(self, goal_vector: list[float], k: int = 10) -> list[NodeMatch]:
        """Find k nearest nodes by feature similarity."""
        params = protocol.query_request(self.domain, limit=k)
        params["goal_vector"] = goal_vector
        params["mode"] = "nearest"
        resp = self._conn.send("query", params)
        return _parse_node_matches(resp)

    def pathfind(
        self,
        from_node: int,
        to_node: int,
        *,
        avoid_flags: list[str] | None = None,
        minimize: str = "hops",
    ) -> Path | None:
        """Find shortest path between nodes."""
        params = protocol.pathfind_request(
            self.domain,
            from_node,
            to_node,
            avoid_flags=avoid_flags,
            minimize=minimize,
        )
        resp = self._conn.send("pathfind", params)

        if "error" in resp:
            code = resp["error"].get("code", "")
            if code == "E_NO_PATH":
                return None
            raise CortexPathError(resp["error"].get("message", "pathfind error"))

        result = resp.get("result", {})
        actions = [
            PathAction(at_node=a["at_node"], opcode=tuple(a["opcode"]))
            for a in result.get("required_actions", [])
        ]
        return Path(
            nodes=result.get("nodes", []),
            total_weight=result.get("total_weight", 0.0),
            hops=result.get("hops", 0),
            required_actions=actions,
        )

    def refresh(
        self,
        *,
        nodes: list[int] | None = None,
        cluster: int | None = None,
        stale_threshold: float | None = None,
    ) -> RefreshResult:
        """Re-render specific nodes and update the map."""
        params = protocol.refresh_request(
            self.domain,
            nodes=nodes,
            cluster=cluster,
            stale_threshold=stale_threshold,
        )
        resp = self._conn.send("refresh", params)
        result = resp.get("result", {})
        return RefreshResult(
            updated_count=result.get("updated_count", 0),
            changed_nodes=result.get("changed_nodes", []),
        )

    def act(
        self,
        node: int,
        opcode: tuple[int, int],
        params: dict[str, Any] | None = None,
        session_id: str | None = None,
    ) -> ActResult:
        """Execute an action on a live page."""
        req_params = protocol.act_request(
            self.domain, node, opcode, params=params, session_id=session_id
        )
        resp = self._conn.send("act", req_params)
        result = resp.get("result", {})
        return ActResult(
            success=result.get("success", False),
            new_url=result.get("new_url"),
            features=result.get("features", {}),
        )

    def watch(
        self,
        *,
        nodes: list[int] | None = None,
        cluster: int | None = None,
        features: list[int] | None = None,
        interval_ms: int = 60000,
    ) -> Iterator[WatchDelta]:
        """Monitor nodes for changes."""
        params = protocol.watch_request(
            self.domain,
            nodes=nodes,
            cluster=cluster,
            features=features,
            interval_ms=interval_ms,
        )
        # Send watch request and stream responses
        self._conn.send("watch", params)
        # In the actual implementation, this would yield deltas
        return iter([])


def _parse_node_matches(resp: dict[str, Any]) -> list[NodeMatch]:
    """Parse node matches from a protocol response."""
    if "error" in resp:
        raise CortexResourceError(resp["error"].get("message", "query error"))

    result = resp.get("result", {})
    matches = result.get("matches", [])
    return [
        NodeMatch(
            index=m.get("index", 0),
            url=m.get("url", ""),
            page_type=m.get("page_type", 0),
            confidence=m.get("confidence", 0.0),
            features=m.get("features", {}),
            similarity=m.get("similarity"),
        )
        for m in matches
    ]

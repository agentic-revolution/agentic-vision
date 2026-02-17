# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Integration tests — require a running Cortex runtime.

These tests are marked with `integration` and skipped by default.
Run with: pytest tests/test_integration.py -m integration
"""

from __future__ import annotations

import os
import socket

import pytest

# Skip entire module unless runtime is reachable
_SOCKET = os.environ.get("CORTEX_SOCKET", "/tmp/cortex.sock")


def _runtime_available() -> bool:
    """Check if the Cortex runtime socket exists and is connectable."""
    if not os.path.exists(_SOCKET):
        return False
    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as s:
            s.settimeout(2.0)
            s.connect(_SOCKET)
            return True
    except OSError:
        return False


pytestmark = pytest.mark.skipif(
    not _runtime_available(),
    reason="Cortex runtime not running",
)


@pytest.mark.integration
def test_status() -> None:
    """Runtime returns a valid status response."""
    import cortex_client

    st = cortex_client.status(socket_path=_SOCKET)
    assert st.version
    assert st.uptime_seconds >= 0


@pytest.mark.integration
def test_map_example() -> None:
    """Map a domain and get a SiteMap with nodes."""
    import cortex_client

    sm = cortex_client.map("example.com", max_render=5, socket_path=_SOCKET)
    assert sm.node_count > 0
    assert sm.domain == "example.com"


@pytest.mark.integration
def test_query_after_map() -> None:
    """Query a mapped domain by page type."""
    import cortex_client

    sm = cortex_client.map("example.com", max_render=5, socket_path=_SOCKET)
    results = sm.filter(page_type=1, limit=5)
    # May be empty if example.com doesn't classify as type 1, but call should succeed
    assert isinstance(results, list)


@pytest.mark.integration
def test_perceive() -> None:
    """Perceive a single URL."""
    import cortex_client

    result = cortex_client.perceive("https://example.com", socket_path=_SOCKET)
    assert result.url == "https://example.com"
    assert result.page_type >= 0


@pytest.mark.integration
def test_pathfind_after_map() -> None:
    """Pathfind between two nodes in a mapped domain."""
    import cortex_client

    sm = cortex_client.map("example.com", max_render=5, socket_path=_SOCKET)
    if sm.node_count >= 2:
        path = sm.pathfind(0, 1)
        # Path may or may not exist depending on edges
        assert path is None or path.hops >= 0

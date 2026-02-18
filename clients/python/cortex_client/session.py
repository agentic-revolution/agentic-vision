# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Session management for authenticated mapping and actions."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class Session:
    """An authenticated session for a domain.

    Sessions are returned by :func:`cortex_client.login`,
    :func:`cortex_client.login_oauth`, and :func:`cortex_client.login_api_key`.
    Pass a session to :func:`cortex_client.map` to map authenticated content.

    Example::

        session = cortex_client.login("example.com", username="me", password="pw")
        site = cortex_client.map("example.com", session=session)
    """

    session_id: str
    domain: str
    auth_type: str
    expires_at: str | None = None

    def __repr__(self) -> str:
        exp = f", expires={self.expires_at}" if self.expires_at else ""
        return (
            f"Session(id={self.session_id!r}, domain={self.domain!r}, "
            f"auth={self.auth_type!r}{exp})"
        )

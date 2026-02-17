# Copyright 2026 Cortex Contributors
# SPDX-License-Identifier: Apache-2.0
"""Exception types for the Cortex client."""


class CortexError(Exception):
    """Base exception for all Cortex errors."""


class CortexConnectionError(CortexError):
    """Cannot connect to the Cortex runtime."""


class CortexTimeoutError(CortexError):
    """Operation timed out."""


class CortexResourceError(CortexError):
    """A requested resource (map, node) was not found."""


class CortexMapError(CortexError):
    """Error during mapping operation."""


class CortexPathError(CortexError):
    """Error during pathfinding operation."""

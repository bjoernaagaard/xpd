"""Which way a tool asks its client for input on the connection of a call."""

from fastmcp import Context
from mcp_types.version import MODERN_PROTOCOL_VERSIONS


def uses_handshake(ctx: Context) -> bool:
    """Whether the client can be asked mid-call. The 2026-07-28 protocol and background tasks
    ask by returning an InputRequiredResult instead."""
    request = ctx.request_context
    return (
        not ctx.is_background_task
        and request is not None
        and request.protocol_version not in MODERN_PROTOCOL_VERSIONS
    )

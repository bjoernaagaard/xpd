"""Launches `python -m xpd.mcp` over stdio for the documentation build scripts."""

import os
import sys

from fastmcp.client.transports import StdioTransport


def stdio_server(root: str | None = None, *flags: str) -> StdioTransport:
    """Transport for `python -m xpd.mcp`, pinned to `root` or reading the client's roots."""
    args = ["-m", "xpd.mcp", *(["--root", root] if root is not None else []), *flags]
    # FastMCP's stdio transport passes a minimal environment by default, so forward ours for
    # XPD_BIN and PYTHONPATH. Without a fixed root, PLUGIN_ROOT makes the server ask the client
    # for its roots on each call.
    env = dict(os.environ)
    if root is None:
        env.pop("XPD_MCP_ROOT", None)
        env["PLUGIN_ROOT"] = ""
    return StdioTransport(command=sys.executable, args=args, env=env)

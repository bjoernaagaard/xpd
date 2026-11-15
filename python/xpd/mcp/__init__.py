"""The xpd MCP server. `xpd mcp` runs it with flags, and `fastmcp run` finds it as
`xpd.mcp:mcp`, configured from the environment."""

from functools import cache
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from fastmcp import FastMCP


@cache
def from_env() -> "FastMCP":
    from xpd.mcp.server import build  # noqa: PLC0415  importing the package must not start one
    from xpd.mcp.settings import Settings  # noqa: PLC0415

    return build(Settings.from_env())


def __getattr__(name: str) -> "FastMCP":
    if name == "mcp":
        return from_env()
    msg = f"module {__name__!r} has no attribute {name!r}"
    raise AttributeError(msg)

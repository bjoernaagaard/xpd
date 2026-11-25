"""The server's settings, from flags or else from the environment, so no wrapper is needed."""

import argparse
import ipaddress
import os
from collections.abc import Sequence
from pathlib import Path
from typing import Literal, NamedTuple

import fastmcp

Transport = Literal["stdio", "http"]
TRUE = {"1", "true", "yes", "on"}


def is_loopback_host(host: str) -> bool:
    cleaned = host.strip().strip("[]").lower()
    if cleaned == "localhost":
        return True
    try:
        return ipaddress.ip_address(cleaned).is_loopback
    except ValueError:
        return False


def default_root() -> Path | None:
    """`XPD_MCP_ROOT`, else the working directory. A client that loads xpd as an agent plugin
    sets `PLUGIN_ROOT` and starts the server in the plugin's folder, so there the root is
    `None` and each call takes it from the client's roots."""
    if named := os.environ.get("XPD_MCP_ROOT"):
        return Path(named)
    return None if "PLUGIN_ROOT" in os.environ else Path.cwd()


class Settings(NamedTuple):
    """`transport`, `host` and `port` stay `None` unless a flag sets them, so FastMCP's own
    FASTMCP_TRANSPORT, FASTMCP_HOST and FASTMCP_PORT apply, with stdio on 127.0.0.1:8000."""

    root: Path | None
    transport: Transport | None = None
    host: str | None = None
    port: int | None = None
    allow_writes: bool = False
    auth_token: str | None = None

    @classmethod
    def from_env(cls) -> "Settings":
        token = os.environ.get("XPD_MCP_AUTH_TOKEN") or None
        return cls(
            root=default_root(),
            allow_writes=os.environ.get("XPD_MCP_ALLOW_WRITES", "").lower() in TRUE,
            auth_token=token,
        )

    @classmethod
    def from_args(cls, argv: Sequence[str] | None = None) -> "Settings":
        env = cls.from_env()
        parser = argparse.ArgumentParser(
            prog="xpd mcp",
            description="The xpd MCP server. It serves stdio unless --transport says otherwise.",
        )
        parser.add_argument(
            "--root",
            type=Path,
            default=env.root,
            help="The project root. Every path a tool gets must be inside it. "
            "Default: XPD_MCP_ROOT, else the working directory. As an agent plugin, "
            "the default is the first root the client names.",
        )
        parser.add_argument(
            "--allow-writes",
            action=argparse.BooleanOptionalAction,
            default=env.allow_writes,
            help="Expose the tools that run the project's scripts over HTTP. "
            "Default: XPD_MCP_ALLOW_WRITES, else off (stdio always allows them).",
        )
        parser.add_argument(
            "--transport",
            choices=["stdio", "http"],
            help="Default: FASTMCP_TRANSPORT, else stdio.",
        )
        parser.add_argument(
            "--host", help="Address to bind for http. Default: FASTMCP_HOST, else 127.0.0.1."
        )
        parser.add_argument(
            "--port", type=int, help="Port for http. Default: FASTMCP_PORT, else 8000."
        )
        args = parser.parse_args(argv)
        return cls(
            root=args.root,
            transport=args.transport,
            host=args.host,
            port=args.port,
            allow_writes=args.allow_writes,
            auth_token=env.auth_token,
        )

    def effective_transport(self) -> str:
        return self.transport or fastmcp.settings.transport

    def effective_host(self) -> str:
        return self.host or fastmcp.settings.host

    def validate_transport(self) -> tuple[str, str]:
        transport = self.effective_transport()
        host = self.effective_host()
        if transport != "stdio":
            if self.root is None:
                msg = (
                    "HTTP transport requires a server-selected project root. "
                    "Set --root or XPD_MCP_ROOT."
                )
                raise ValueError(msg)
            if not is_loopback_host(host) and not self.auth_token:
                msg = (
                    f"HTTP transport on non-loopback host {host!r} requires "
                    "XPD_MCP_AUTH_TOKEN to authenticate clients."
                )
                raise ValueError(msg)
        return transport, host

    def transport_options(self) -> dict[str, str | int]:
        options = {"host": self.host, "port": self.port}
        return {key: value for key, value in options.items() if value is not None}

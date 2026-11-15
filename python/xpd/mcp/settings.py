"""The server's settings, from flags or else from the environment, so no wrapper is needed."""

import argparse
import os
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Literal

Transport = Literal["stdio", "http"]
TRUE = {"1", "true", "yes", "on"}


def default_root() -> Path | None:
    """`XPD_MCP_ROOT`, else the working directory. A client that loads xpd as an agent plugin
    sets `PLUGIN_ROOT` and starts the server in the plugin's folder, so there the root is
    `None` and each call takes it from the client's roots."""
    if named := os.environ.get("XPD_MCP_ROOT"):
        return Path(named)
    return None if "PLUGIN_ROOT" in os.environ else Path.cwd()


@dataclass(frozen=True)
class Settings:
    """`transport`, `host` and `port` stay `None` unless a flag sets them, so FastMCP's own
    FASTMCP_TRANSPORT, FASTMCP_HOST and FASTMCP_PORT apply, with stdio on 127.0.0.1:8000."""

    root: Path | None
    code_mode: bool
    transport: Transport | None = None
    host: str | None = None
    port: int | None = None

    @classmethod
    def from_env(cls) -> "Settings":
        return cls(
            root=default_root(),
            code_mode=os.environ.get("XPD_MCP_CODE_MODE", "").lower() in TRUE,
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
            "--code-mode",
            action=argparse.BooleanOptionalAction,
            default=env.code_mode,
            help="Show the tools through FastMCP's code mode, as search, get_schema and execute. "
            "Default: XPD_MCP_CODE_MODE, else off.",
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
            code_mode=args.code_mode,
            transport=args.transport,
            host=args.host,
            port=args.port,
        )

    def transport_options(self) -> dict[str, str | int]:
        options = {"host": self.host, "port": self.port}
        return {key: value for key, value in options.items() if value is not None}

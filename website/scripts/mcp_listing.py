"""Prints what the xpd MCP server lists, as JSON, for the site's MCP reference.

It starts the server the way a client does, as `python -m xpd.mcp` over stdio, and asks it for
its tools, resources, resource templates and prompts. It also prints the server's own `--help`, which lists its flags.
"""

import asyncio
import json
import os
import subprocess
import sys
import tempfile
from typing import Any

from fastmcp import Client
from mcp_server import stdio_server
from pydantic import BaseModel


def dump(items: list[BaseModel]) -> list[dict[str, Any]]:
    return [item.model_dump(mode="json", by_alias=True, exclude_none=True) for item in items]


async def listing(root: str) -> dict[str, Any]:
    async with Client(stdio_server(root)) as client:
        info = client.server_info
        return {
            "server": info.model_dump(mode="json", exclude_none=True) if info else None,
            "instructions": client.instructions,
            "tools": dump(await client.list_tools()),
            "resources": dump(await client.list_resources()),
            "resource_templates": dump(await client.list_resource_templates()),
            "prompts": dump(await client.list_prompts()),
        }


def main() -> None:
    # An empty root, so the listing holds no rules of the project that builds the site.
    with tempfile.TemporaryDirectory() as root:
        default = asyncio.run(listing(root))
    help_text = subprocess.run(
        [sys.executable, "-m", "xpd.mcp", "--help"],
        capture_output=True,
        text=True,
        check=True,
        env={**os.environ, "COLUMNS": "100"},
    ).stdout
    json.dump({**default, "help": help_text}, sys.stdout)


if __name__ == "__main__":
    main()

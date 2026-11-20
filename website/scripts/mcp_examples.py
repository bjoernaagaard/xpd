"""Makes the tool calls that the MCP reference shows as examples, and prints their answers as JSON.

It reads one JSON call per line on stdin, each with a `root`, a `tool` and its `arguments`, and
writes one JSON answer per line on stdout. It starts one `python -m xpd.mcp` server over stdio and
supplies each call's `root` through the MCP roots callback. The text of a tool that also draws a
view is its report. The client declines every question that a tool asks the user.
"""

import asyncio
import json
import sys
from pathlib import Path
from typing import TypedDict

from fastmcp import Client
from fastmcp.client.elicitation import ElicitResult
from mcp_server import stdio_server


class Call(TypedDict):
    root: str
    tool: str
    arguments: dict[str, object]


class Answer(TypedDict):
    text: str
    failed: bool


async def decline(*_: object) -> ElicitResult:
    return ElicitResult(action="decline")


async def serve() -> None:
    current_root: Path | None = None

    async def roots(_context: object) -> list[str]:
        return [current_root.as_uri()] if current_root is not None else []

    async with Client(stdio_server(), roots=roots, elicitation_handler=decline) as client:
        while line := await asyncio.to_thread(sys.stdin.readline):
            if not line.strip():
                continue
            call: Call = json.loads(line)
            current_root = Path(call["root"])
            result = await client.call_tool(call["tool"], call["arguments"], raise_on_error=False)
            text = "".join(getattr(part, "text", "") for part in result.content)
            sys.stdout.write(f"{json.dumps(Answer(text=text, failed=result.is_error))}\n")
            sys.stdout.flush()


def main() -> None:
    asyncio.run(serve())


if __name__ == "__main__":
    main()

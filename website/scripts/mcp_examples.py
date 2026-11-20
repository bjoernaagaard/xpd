"""Makes the tool calls that the MCP reference shows as examples, and prints their answers as JSON.

It reads a JSON list of calls on stdin, each with a `root`, a `tool` and its `arguments`. It starts
the server the way a client does, as `python -m xpd.mcp` over stdio, and prints one answer for
each call, with the text that the tool returned and whether the call failed. The text of a tool
that also draws a view is its report. The client declines every question that a tool asks the
user.
"""

import asyncio
import json
import os
import sys
from typing import TypedDict

from fastmcp import Client
from fastmcp.client.elicitation import ElicitResult
from fastmcp.client.transports import StdioTransport


class Call(TypedDict):
    root: str
    tool: str
    arguments: dict[str, object]


class Answer(TypedDict):
    text: str
    failed: bool


async def decline(*_: object) -> ElicitResult:
    return ElicitResult(action="decline")


async def answer(call: Call) -> Answer:
    server = StdioTransport(
        command=sys.executable,
        args=["-m", "xpd.mcp", "--root", call["root"]],
        env=dict(os.environ),
    )
    async with Client(server, elicitation_handler=decline) as client:
        result = await client.call_tool(call["tool"], call["arguments"], raise_on_error=False)
    text = "".join(getattr(part, "text", "") for part in result.content)
    return Answer(text=text, failed=result.is_error)


async def answers(calls: list[Call]) -> list[Answer]:
    return [await answer(call) for call in calls]


def main() -> None:
    calls: list[Call] = json.load(sys.stdin)
    json.dump(asyncio.run(answers(calls)), sys.stdout)


if __name__ == "__main__":
    main()

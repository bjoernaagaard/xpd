"""Shared behavior of xpd's MCP tools."""

from collections.abc import Awaitable, Callable, Sequence
from functools import wraps

from fastmcp import Context
from mcp.types import (
    InputRequiredResult,
    ToolAnnotations,
)

from xpd.mcp.binary import Command, Xpd
from xpd.mcp.root import Root, RootsRequiredError


def rooted[**P, R](
    tool: Callable[P, Awaitable[R]],
) -> Callable[P, Awaitable[R | InputRequiredResult]]:
    """Return the roots request on protocols that resume tools in another round."""

    @wraps(tool)
    async def call(*args: P.args, **kwargs: P.kwargs) -> R | InputRequiredResult:
        try:
            return await tool(*args, **kwargs)
        except RootsRequiredError as error:
            return error.request

    return call


# Reads the project's files and changes nothing, so a client may run it without asking.
READS = ToolAnnotations(
    readOnlyHint=True, destructiveHint=False, idempotentHint=True, openWorldHint=False
)
# Runs the user's scripts, which may do anything a Python script does.
RUNS_CODE = ToolAnnotations(
    readOnlyHint=False, destructiveHint=True, idempotentHint=False, openWorldHint=True
)


async def run[T](
    ctx: Context, xpd: Xpd, command: Command[T], root: Root, paths: Sequence[str], *flags: str
) -> T:
    """Runs one command in the root with progress and a log line, which also reach a background
    task."""
    shown = " ".join(["xpd", *command.args[:1], *flags, *paths])
    await ctx.report_progress(0, 1, shown)
    await ctx.info(f"running {shown}")
    result = await xpd.run(command, root.path, paths, *flags, project_root=root.path)
    await ctx.report_progress(1, 1, f"{shown} finished")
    return result

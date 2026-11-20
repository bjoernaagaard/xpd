"""Shared behavior of xpd's MCP tools."""

from collections.abc import Awaitable, Callable, Sequence
from functools import wraps
from typing import ParamSpec, TypeVar

from fastmcp import Context
from mcp.types import (
    ElicitRequest,
    ElicitRequestFormParams,
    ElicitResult,
    InputRequiredResult,
    ToolAnnotations,
)

from xpd.mcp.binary import Command, T, Xpd
from xpd.mcp.protocol import uses_handshake
from xpd.mcp.root import Project, Root, RootsRequiredError

P = ParamSpec("P")
R = TypeVar("R")


def rooted(tool: Callable[P, Awaitable[R]]) -> Callable[P, Awaitable[R | InputRequiredResult]]:
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
# Fixes rewrite files in place. Fixing repeats until nothing changes, so a second call with the
# same arguments changes nothing more.
WRITES = ToolAnnotations(
    readOnlyHint=False, destructiveHint=True, idempotentHint=True, openWorldHint=False
)
# A structural rewrite whose result still matches its pattern rewrites the code again on a
# second call with the same arguments.
REWRITES = ToolAnnotations(
    readOnlyHint=False, destructiveHint=True, idempotentHint=False, openWorldHint=False
)
# Runs the user's scripts, which may do anything a Python script does.
RUNS_CODE = ToolAnnotations(
    readOnlyHint=False, destructiveHint=True, idempotentHint=False, openWorldHint=True
)


async def run(
    ctx: Context, xpd: Xpd, command: Command[T], root: Root, paths: Sequence[str], *flags: str
) -> T:
    """Runs one command in the root with progress and a log line, which also reach a background
    task."""
    shown = " ".join(["xpd", *command.args[:1], *flags, *paths])
    await ctx.report_progress(0, 1, shown)
    await ctx.info(f"running {shown}")
    result = await xpd.run(command, root.path, paths, *flags)
    await ctx.report_progress(1, 1, f"{shown} finished")
    return result


def ask_to_confirm(message: str, key: str, title: str) -> InputRequiredResult:
    params = ElicitRequestFormParams(
        message=message,
        requested_schema={
            "type": "object",
            "properties": {"confirm": {"type": "boolean", "title": title}},
            "required": ["confirm"],
        },
    )
    return InputRequiredResult(
        result_type="input_required",
        input_requests={key: ElicitRequest(method="elicitation/create", params=params)},
    )


async def confirmed(
    ctx: Context, project: Project, message: str, *, key: str, title: str
) -> bool | InputRequiredResult:
    """The user's answer to `message`, or the request that asks for it on this protocol."""
    if uses_handshake(ctx):
        answer = await ctx.elicit(message, response_type=bool, response_title="Apply")
        return answer.action == "accept" and bool(answer.data)
    responses = ctx.input_responses
    if responses is None or key not in responses:
        request = ask_to_confirm(message, key, title)
        requests = {**(request.input_requests or {}), **project.roots_request(ctx)}
        return request.model_copy(update={"input_requests": requests})
    answer = responses[key]
    if not isinstance(answer, ElicitResult) or answer.action != "accept" or not answer.content:
        return False
    # Only the boolean counts, so a text answer such as "false" does not confirm.
    return answer.content.get("confirm") is True

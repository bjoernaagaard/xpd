"""The tools that run xpd on the project's files."""

from fastmcp import Context, FastMCP
from mcp.types import (
    ElicitRequest,
    ElicitRequestFormParams,
    ElicitResult,
    InputRequiredResult,
    ToolAnnotations,
)
from mcp_types.version import MODERN_PROTOCOL_VERSIONS

from xpd.mcp.binary import CHECK, EQUIV, GRAPH, PROFILE, Command, T, Xpd
from xpd.mcp.models import CheckReport, Equivalence, GraphReport, Profile
from xpd.mcp.root import Root

# Reads the project's files and changes nothing, so a client may run it without asking.
READS = ToolAnnotations(
    readOnlyHint=True, destructiveHint=False, idempotentHint=True, openWorldHint=False
)
# Fixes rewrite files in place. Fixing repeats until nothing changes, so a second call with the
# same arguments changes nothing more.
WRITES = ToolAnnotations(
    readOnlyHint=False, destructiveHint=True, idempotentHint=True, openWorldHint=False
)
# Runs the user's scripts, which may do anything a Python script does.
RUNS_CODE = ToolAnnotations(
    readOnlyHint=False, destructiveHint=True, idempotentHint=False, openWorldHint=True
)
# The key of the answer to the unsafe-fixes question, on the modern protocol.
CONFIRM = "confirm_unsafe_fixes"


async def run(ctx: Context, xpd: Xpd, command: Command[T], *args: str) -> T:
    """Runs one command with progress and a log line, which also reach a background task."""
    shown = " ".join(["xpd", *command.args[:1], *args])
    await ctx.report_progress(0, 1, shown)
    await ctx.info(f"running {shown}")
    result = await xpd.run(command, *args)
    await ctx.report_progress(1, 1, f"{shown} finished")
    return result


def uses_handshake(ctx: Context) -> bool:
    """Whether the client can be asked mid-call with ctx.elicit. The 2026-07-28 protocol and
    background tasks ask by returning an InputRequiredResult instead."""
    request = ctx.request_context
    return (
        not ctx.is_background_task
        and request is not None
        and request.protocol_version not in MODERN_PROTOCOL_VERSIONS
    )


def ask_to_confirm(message: str) -> InputRequiredResult:
    params = ElicitRequestFormParams(
        message=message,
        requested_schema={
            "type": "object",
            "properties": {"confirm": {"type": "boolean", "title": "Apply unsafe fixes"}},
            "required": ["confirm"],
        },
    )
    return InputRequiredResult(
        result_type="input_required",
        input_requests={CONFIRM: ElicitRequest(method="elicitation/create", params=params)},
    )


async def confirmed(ctx: Context, message: str) -> bool | InputRequiredResult:
    """The user's answer to `message`, or the request that asks for it on this protocol."""
    if uses_handshake(ctx):
        answer = await ctx.elicit(message, response_type=bool, response_title="Apply")
        return answer.action == "accept" and bool(answer.data)
    responses = ctx.input_responses
    if responses is None or CONFIRM not in responses:
        return ask_to_confirm(message)
    answer = responses[CONFIRM]
    if not isinstance(answer, ElicitResult) or answer.action != "accept" or not answer.content:
        return False
    return bool(answer.content.get("confirm"))


def register(mcp: FastMCP, xpd: Xpd, root: Root) -> None:
    @mcp.tool(tags={"lint"}, annotations=READS, task=True)
    async def check(
        ctx: Context, paths: list[str] | None = None, *, unsafe_fixes: bool = False
    ) -> CheckReport:
        """Lint Python files for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn,
        Apache Hamilton and Python itself. Changes no file.

        Run it after you write or change Python. Fix every `error`. Fix a `warning` unless you
        can say why the code is right here, and don't silence rules. An issue whose `fix` is set
        can be fixed with the `fix` tool. A `missing-<package>` warning names a package the
        project lacks, so ask the user before you add the dependency. The `rule` tool explains
        any rule with examples.

        Args:
            paths: Files or folders inside the project root. Folders respect .gitignore.
                Default: the whole root.
            unsafe_fixes: Count unsafe fixes, which may change behavior, in `fixable` too.
        """
        args = root.all_inside(paths or ["."])
        flags = ["--unsafe-fixes"] if unsafe_fixes else []
        return await run(ctx, xpd, CHECK, *flags, *args)

    @mcp.tool(tags={"lint", "fix"}, annotations=WRITES, task=True)
    async def fix(
        ctx: Context, paths: list[str] | None = None, *, unsafe: bool = False
    ) -> CheckReport | InputRequiredResult:
        """Write xpd's fixes into the files, then lint what is left.

        Safe fixes keep what the code does. With `unsafe`, the user is asked to confirm first,
        because an unsafe fix may change behavior. Read the diff after unsafe fixes.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
            unsafe: Also apply unsafe fixes, after the user confirms.
        """
        args = root.all_inside(paths or ["."])
        if unsafe:
            preview = await run(ctx, xpd, CHECK, "--unsafe-fixes", *args)
            unsafe_count = sum(issue.fix == "unsafe" for issue in preview.issues)
            if unsafe_count:
                where = ", ".join(args)
                answer = await confirmed(
                    ctx,
                    f"Apply {unsafe_count} unsafe fixes in {where}? "
                    "An unsafe fix may change what the code does.",
                )
                if isinstance(answer, InputRequiredResult):
                    return answer
                if not answer:
                    await ctx.info("unsafe fixes declined, so only safe fixes are written")
                    unsafe = False
        flags = ["--fix", "--unsafe-fixes"] if unsafe else ["--fix"]
        return await run(ctx, xpd, CHECK, *flags, *args)

    @mcp.tool(tags={"dataflow"}, annotations=READS)
    async def graph(ctx: Context, paths: list[str]) -> GraphReport:
        """Read the dataflow of step code or Apache Hamilton modules without running them: which
        step or node reads which, and what does not connect. Changes no file.

        Run it after you add, move or rename a step or a node. Fix every `error`, and fix a
        `warning` unless you can say why the code is right. Before you add a column to step
        code, read `pipelines[].steps` and reuse a column that exists. For Hamilton, pass all
        modules of one Driver together. `inputs` are what the caller passes.

        Args:
            paths: Files or folders inside the project root.
        """
        return await run(ctx, xpd, GRAPH, *root.all_inside(paths))

    @mcp.tool(tags={"data"}, annotations=READS, task=True)
    async def profile(ctx: Context, file: str) -> Profile:
        """Profile a data file (csv, parquet, feather, json or xlsx): each column's dtype,
        nulls, distinct values and memory. Run it before you choose methods for the data.
        Needs pandas in the project's Python. Changes no file.

        Args:
            file: The data file, inside the project root.
        """
        return await run(ctx, xpd, PROFILE, root.inside(file))

    @mcp.tool(tags={"data", "refactor"}, annotations=RUNS_CODE, task=True)
    async def equiv(ctx: Context, old: str, new: str) -> Equivalence:
        """Run two scripts with the project's Python and check that they leave an equal
        `result` (DataFrames and Series with pandas.testing, anything else with
        numpy.testing). A refactor of data logic is done when this says `equivalent: true`.
        Both scripts must assign `result`. Needs pandas in the project's Python.

        Args:
            old: The script before the refactor, inside the project root.
            new: The script after the refactor, inside the project root.
        """
        return await run(ctx, xpd, EQUIV, root.inside(old), root.inside(new))

"""The tools that run xpd on the project's files."""

from collections.abc import Sequence

from fastmcp import Context, FastMCP
from mcp.types import (
    ElicitRequest,
    ElicitRequestFormParams,
    ElicitResult,
    InputRequiredResult,
    ToolAnnotations,
)

from xpd.mcp.binary import CHECK, EQUIV, GRAPH, PROFILE, Command, T, Xpd
from xpd.mcp.models import CheckReport, Equivalence, GraphReport, Profile
from xpd.mcp.protocol import uses_handshake
from xpd.mcp.root import Project, Root

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
    # Only the boolean counts, so a text answer such as "false" does not confirm.
    return answer.content.get("confirm") is True


async def unsafe_confirmed(
    ctx: Context, xpd: Xpd, project: Project, root: Root, paths: Sequence[str]
) -> bool | InputRequiredResult:
    """Whether `fix` may write the unsafe fixes in `paths`. It asks the user only when there
    are some."""
    preview = await run(ctx, xpd, CHECK, root, paths, "--unsafe-fixes")
    count = sum(issue.fix == "unsafe" for issue in preview.issues)
    if not count:
        return True
    answer = await confirmed(
        ctx,
        f"Apply {count} unsafe fixes in {', '.join(paths)}? "
        "An unsafe fix may change what the code does.",
    )
    if isinstance(answer, InputRequiredResult):
        # Each round starts over, so the question also asks for the roots again.
        requests = {**(answer.input_requests or {}), **project.roots_request(ctx)}
        return answer.model_copy(update={"input_requests": requests})
    if not answer:
        await ctx.info("unsafe fixes declined, so only safe fixes are written")
    return answer


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    register_lint(mcp, xpd, project)
    register_data(mcp, xpd, project)


def register_lint(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"lint"}, annotations=READS, task=True)
    async def check(
        ctx: Context, paths: list[str] | None = None, *, unsafe_fixes: bool = False
    ) -> CheckReport | InputRequiredResult:
        """Lint Python files for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn,
        scikit-lego, MAPIE, Category Encoders, linearmodels, Apache Hamilton and Python itself.
        Changes no file.

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
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        args = root.all_inside(paths or ["."])
        flags = ["--unsafe-fixes"] if unsafe_fixes else []
        return await run(ctx, xpd, CHECK, root, args, *flags)

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
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        args = root.all_inside(paths or ["."])
        if unsafe:
            answer = await unsafe_confirmed(ctx, xpd, project, root, args)
            if isinstance(answer, InputRequiredResult):
                return answer
            unsafe = answer
        flags = ["--fix", "--unsafe-fixes"] if unsafe else ["--fix"]
        return await run(ctx, xpd, CHECK, root, args, *flags)


def register_data(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"dataflow"}, annotations=READS)
    async def graph(ctx: Context, paths: list[str]) -> GraphReport | InputRequiredResult:
        """Read the dataflow of step code or Apache Hamilton modules without running them: which
        step or node reads which, and what does not connect. Changes no file.

        Run it after you add, move or rename a step or a node. Fix every `error`, and fix a
        `warning` unless you can say why the code is right. Before you add a column to step
        code, read `pipelines[].steps` and reuse a column that exists. For Hamilton, pass all
        modules of one Driver together. `inputs` are what the caller passes.

        Args:
            paths: Files or folders inside the project root.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        return await run(ctx, xpd, GRAPH, root, root.all_inside(paths))

    @mcp.tool(tags={"data"}, annotations=READS, task=True)
    async def profile(ctx: Context, file: str) -> Profile | InputRequiredResult:
        """Profile a data file (csv, parquet, feather, json or xlsx): each column's dtype,
        nulls, distinct values and memory. Run it before you choose methods for the data.
        Needs pandas in the project's Python. Changes no file.

        Args:
            file: The data file, inside the project root.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        return await run(ctx, xpd, PROFILE, root, [root.inside(file)])

    @mcp.tool(tags={"data", "refactor"}, annotations=RUNS_CODE, task=True)
    async def equiv(ctx: Context, old: str, new: str) -> Equivalence | InputRequiredResult:
        """Run two scripts with the project's Python, each in its own process, and check that
        they leave an equal `result` (DataFrames and Series with pandas.testing, anything else
        with numpy.testing). A refactor of data logic is done when this says
        `equivalent: true`. Both scripts must assign `result`, and pickle must be able to save
        it. Needs pandas in the project's Python.

        Args:
            old: The script before the refactor, inside the project root.
            new: The script after the refactor, inside the project root.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        return await run(ctx, xpd, EQUIV, root, [root.inside(old), root.inside(new)])

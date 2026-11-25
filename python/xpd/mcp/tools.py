"""The tools that run xpd on the project's files."""

from collections.abc import Sequence
from typing import NamedTuple

from fastmcp import Context, FastMCP
from mcp.types import (
    InputRequiredResult,
)

from xpd.mcp.base import READS, RUNS_CODE, WRITES, confirmed, rooted, run
from xpd.mcp.base import run as xpd_run
from xpd.mcp.binary import CHART_REVIEW, CHECK, EQUIV, GRAPH, PROFILE, REFIT, UNTANGLE, Xpd
from xpd.mcp.models import (
    ChartReview,
    CheckReport,
    Equivalence,
    GraphReport,
    Ledger,
    Profile,
    Refit,
)
from xpd.mcp.names import ToolName
from xpd.mcp.options import CheckOptions
from xpd.mcp.root import Project


async def unsafe_confirmed(
    ctx: Context, project: Project, paths: Sequence[str], preview: CheckReport
) -> bool | InputRequiredResult:
    """Ask only when the selected rules propose unsafe fixes."""
    count = sum(issue.fix == "unsafe" for issue in preview.issues)
    if not count:
        return True
    answer = await confirmed(
        ctx,
        project,
        f"Apply {count} unsafe fixes in {', '.join(paths)}? "
        "An unsafe fix may change what the code does.",
        key="confirm_unsafe_fixes",
        title="Apply unsafe fixes",
    )
    if isinstance(answer, InputRequiredResult):
        return answer
    if not answer:
        await ctx.info("unsafe fixes declined, so only safe fixes are written")
    return answer


class Analysis(NamedTuple):
    xpd: Xpd
    project: Project

    @rooted
    async def check(
        self,
        ctx: Context,
        paths: list[str] | None = None,
        *,
        unsafe_fixes: bool = False,
        options: CheckOptions | None = None,
    ) -> CheckReport | InputRequiredResult:
        """Lint Python files for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn,
        scikit-lego, MAPIE, Category Encoders, linearmodels, arch, pymoo, plotly, Apache Hamilton
        and Python itself. Changes no file.

        Run it after you write or change Python. Fix every `error`. Fix a `warning` unless you
        can say why the code is right here, and don't silence rules. An issue whose `fix` is set
        can be fixed with the `fix` tool. A `pandas/missing-<package>` warning names a package the
        project lacks, so ask the user before you add the dependency. The `rule` tool explains
        any rule with examples.

        Args:
            paths: Files or folders inside the project root. Folders respect .gitignore.
                Default: the whole root.
            unsafe_fixes: Count unsafe fixes, which may change behavior, in `fixable` too.
            options: Which rules run, and the project file that adds the project's own rules.
        """
        root = await self.project.required_root(ctx)
        args = root.all_inside(paths or ["."])
        flags = (options or CheckOptions()).flags(root)
        if unsafe_fixes:
            flags.append("--unsafe-fixes")
        return await run(ctx, self.xpd, CHECK, root, args, *flags)

    @rooted
    async def graph(self, ctx: Context, paths: list[str]) -> GraphReport | InputRequiredResult:
        """Read the dataflow of step code or Apache Hamilton modules without running them: which
        step or node reads which, and what does not connect. Changes no file.

        Run it after you add, move or rename a step or a node. Fix every `error`, and fix a
        `warning` unless you can say why the code is right. Before you add a column to step
        code, read `pipelines[].steps` and reuse a column that exists. For Hamilton, pass all
        modules of one Driver together. `inputs` are what the caller passes.

        Args:
            paths: Files or folders inside the project root.
        """
        root = await self.project.required_root(ctx)
        return await run(ctx, self.xpd, GRAPH, root, root.all_inside(paths))

    @rooted
    async def untangle(
        self,
        ctx: Context,
        paths: list[str] | None = None,
        kinds: list[str] | None = None,
        limit: int = 20,
        since: str | None = None,
    ) -> Ledger | InputRequiredResult:
        """List the places where the code gets wrong which pieces change together, read from
        the files and the git history without running anything. Changes no file.

        Run it before a refactor of a codebase and after each round of it. `summary` counts each
        kind in four groups: hidden links, needless links, surplus and special cases. `findings`
        holds the strongest of each kind with its places and the evidence. Copies and files that
        change together are found in every language, and the rest is read from Python. Judge
        each finding against the code, since a count is evidence and not a verdict.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
            kinds: List only these kinds, such as `copy` or `import-cycle`.
            limit: The most findings to list of each kind. 0 lists every finding.
            since: A commit to compare with. Each count gets `was`, each finding that the
                commit did not have gets `new`, and `fixed` lists the findings that are gone.
        """
        root = await self.project.required_root(ctx)
        flags = ["--limit", str(limit)]
        if kinds:
            flags += ["--kind", ",".join(kinds)]
        if since is not None:
            flags += ["--since", since]
        return await run(ctx, self.xpd, UNTANGLE, root, root.all_inside(paths or ["."]), *flags)

    @rooted
    async def profile(self, ctx: Context, file: str) -> Profile | InputRequiredResult:
        """Profile a data file (csv, parquet, feather, json or xlsx): each column's dtype,
        nulls, distinct values and memory. Run it before you choose methods for the data.
        Needs pandas in the project's Python. Changes no file.

        Args:
            file: The data file, inside the project root.
        """
        root = await self.project.required_root(ctx)
        return await run(ctx, self.xpd, PROFILE, root, [root.inside(file)])


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> Analysis:
    analysis = Analysis(xpd, project)
    register_lint(mcp, xpd, project, analysis)
    register_data(mcp, xpd, project, analysis)
    return analysis


def register_lint(mcp: FastMCP, xpd: Xpd, project: Project, analysis: Analysis) -> None:
    mcp.tool(name=ToolName.CHECK, tags={"lint"}, annotations=READS, task=True)(analysis.check)

    @mcp.tool(name=ToolName.FIX, tags={"lint", "fix"}, annotations=WRITES, task=True)
    @rooted
    async def fix(
        ctx: Context,
        paths: list[str] | None = None,
        *,
        unsafe: bool = False,
        options: CheckOptions | None = None,
    ) -> CheckReport | InputRequiredResult:
        """Write xpd's fixes into the files, then lint what is left.

        Safe fixes keep what the code does. With `unsafe`, the user is asked to confirm first,
        because an unsafe fix may change behavior. Read the diff after unsafe fixes.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
            unsafe: Also apply unsafe fixes, after the user confirms.
            options: Which rules run, and the project file that adds the project's own rules.
        """
        root = await project.required_root(ctx)
        args = root.all_inside(paths or ["."])
        flags = (options or CheckOptions()).flags(root)
        if unsafe:
            preview = await run(ctx, xpd, CHECK, root, args, "--unsafe-fixes", *flags)
            answer = await unsafe_confirmed(ctx, project, args, preview)
            if isinstance(answer, InputRequiredResult):
                return answer
            unsafe = answer
        flags.extend(["--fix", "--unsafe-fixes"] if unsafe else ["--fix"])
        return await run(ctx, xpd, CHECK, root, args, *flags)

    @mcp.tool(name=ToolName.CHART_REVIEW, tags={"lint", "chart"}, annotations=RUNS_CODE, task=True)
    @rooted
    async def chart_review(
        ctx: Context, path: str, run: str | None = None
    ) -> ChartReview | InputRequiredResult:
        """Review the charts of a file against Edward Tufte's book "The Visual Display of
        Quantitative Information", for a chart in plotly, Dash, matplotlib, seaborn or pandas.
        Call it when you make, change or review a chart. Changes no file.

        It returns each chart call with its kind and the plotly call that draws it, the plotly
        rules that fire on the file, and each question of the book that applies to a chart,
        with its page, what to look at and the plotly code to write. Answer each question from
        the chart and its data, and redraw a chart in another library with its plotly route.

        Args:
            path: The Python file that draws the charts, inside the project root.
            run: A Python file inside the project root that defines `figures()`, which returns
                a dict of name to figure. It runs project code in the project's Python, after
                the user confirms, and each measured fact goes under the questions it answers.
        """
        root = await project.required_root(ctx)
        args = [root.inside(path)]
        if run is None:
            return await xpd_run(ctx, xpd, CHART_REVIEW, root, args)
        answer = await confirmed(
            ctx,
            project,
            f"Run {run} with the project's Python to measure the figures it returns? "
            "It runs project code.",
            key="confirm_chart_run",
            title="Run the adapter",
        )
        if isinstance(answer, InputRequiredResult):
            return answer
        if not answer:
            await ctx.info("the run was declined, so the review has no measured facts")
            return await xpd_run(ctx, xpd, CHART_REVIEW, root, args)
        return await xpd_run(ctx, xpd, CHART_REVIEW, root, args, "--run", root.inside(run))


def register_data(mcp: FastMCP, xpd: Xpd, project: Project, analysis: Analysis) -> None:
    mcp.tool(name=ToolName.GRAPH, tags={"dataflow"}, annotations=READS)(analysis.graph)

    mcp.tool(name=ToolName.UNTANGLE, tags={"structure"}, annotations=READS, task=True)(
        analysis.untangle
    )

    mcp.tool(name=ToolName.PROFILE, tags={"data"}, annotations=READS, task=True)(analysis.profile)

    @mcp.tool(name=ToolName.REFIT, tags={"pipeline"}, annotations=RUNS_CODE, task=True)
    @rooted
    async def refit(ctx: Context, file: str) -> Refit | InputRequiredResult:
        """Fit a clone of the saved Pipeline on raw training rows and compare held out
        outputs and validation scores. Load only a model you trust because joblib runs code.
        Needs pandas and scikit-learn in the project's Python. Changes no saved file.

        Args:
            file: The contract, a JSON file inside the project root.
        """
        root = await project.required_root(ctx)
        return await run(ctx, xpd, REFIT, root, [root.inside(file)])

    @mcp.tool(name=ToolName.EQUIV, tags={"data", "refactor"}, annotations=RUNS_CODE, task=True)
    @rooted
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
        root = await project.required_root(ctx)
        return await run(ctx, xpd, EQUIV, root, [root.inside(old), root.inside(new)])

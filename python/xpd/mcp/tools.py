"""The tools that run xpd on the project's files."""

from collections.abc import Sequence
from pathlib import Path

from fastmcp import Context, FastMCP
from mcp.types import (
    InputRequiredResult,
)
from pydantic import BaseModel, Field

from xpd.mcp.base import READS, RUNS_CODE, WRITES, confirmed, rooted, run
from xpd.mcp.binary import API, CHECK, EQUIV, FIND, GRAPH, PROFILE, REFIT, Xpd
from xpd.mcp.models import (
    CheckReport,
    DiscoveryEntry,
    DiscoveryMatches,
    Equivalence,
    GraphReport,
    Profile,
    Refit,
    Severity,
)
from xpd.mcp.root import Project, Root


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


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    register_discovery(mcp, xpd)
    register_lint(mcp, xpd, project)
    register_data(mcp, xpd, project)


class CheckOptions(BaseModel):
    rules: list[str] = Field(default_factory=list, description="Packages or public rule names.")
    severity: Severity | None = Field(
        default=None, description="Run only the rules of this severity and above."
    )
    config: str | None = Field(
        default=None,
        description="A project file to read instead of the nearest sgconfig.yml, relative to the "
        "project root.",
    )

    def flags(self, root: Root) -> list[str]:
        flags = [f"--rule={name}" for name in self.rules]
        if self.severity is not None:
            flags.append(f"--severity={self.severity}")
        if self.config is not None:
            flags.append(f"--config={root.inside(self.config)}")
        return flags


def register_lint(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"lint"}, annotations=READS, task=True)
    @rooted
    async def check(
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
        root = await project.required_root(ctx)
        args = root.all_inside(paths or ["."])
        flags = (options or CheckOptions()).flags(root)
        if unsafe_fixes:
            flags.append("--unsafe-fixes")
        return await run(ctx, xpd, CHECK, root, args, *flags)

    @mcp.tool(tags={"lint", "fix"}, annotations=WRITES, task=True)
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


def register_data(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"dataflow"}, annotations=READS)
    @rooted
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
        root = await project.required_root(ctx)
        return await run(ctx, xpd, GRAPH, root, root.all_inside(paths))

    @mcp.tool(tags={"data"}, annotations=READS, task=True)
    @rooted
    async def profile(ctx: Context, file: str) -> Profile | InputRequiredResult:
        """Profile a data file (csv, parquet, feather, json or xlsx): each column's dtype,
        nulls, distinct values and memory. Run it before you choose methods for the data.
        Needs pandas in the project's Python. Changes no file.

        Args:
            file: The data file, inside the project root.
        """
        root = await project.required_root(ctx)
        return await run(ctx, xpd, PROFILE, root, [root.inside(file)])

    @mcp.tool(tags={"pipeline"}, annotations=RUNS_CODE, task=True)
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

    @mcp.tool(tags={"data", "refactor"}, annotations=RUNS_CODE, task=True)
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


def register_discovery(mcp: FastMCP, xpd: Xpd) -> None:
    @mcp.tool(tags={"discovery"}, annotations=READS)
    async def find(
        task: str, package: str | None = None, kind: str | None = None, limit: int | None = None
    ) -> DiscoveryMatches:
        """Find up to five symbols, three rules and three recipes, ranked within each kind.

        Search before choosing a library, even when it is not installed. Describe the task in
        your own words, because search ranks by meaning as well as by keywords.
        Read the chosen symbol with api before writing code.

        Args:
            task: The task in your own words, such as "grouped regression".
            package: Search only this library, by its Python import name, such as sklearn.
            kind: Search only `symbol`, `rule` or `recipe` entries.
            limit: The most entries of each kind. Default: 5 symbols, 3 rules and 3 recipes.
        """
        flags = []
        if limit is not None:
            flags.append(f"--limit={limit}")
        if package:
            flags.append(f"--package={package}")
        if kind:
            flags.append(f"--kind={kind}")
        return await xpd.run(FIND, Path.cwd(), [task], *flags)

    @mcp.tool(tags={"discovery"}, annotations=READS)
    async def api(name: str) -> DiscoveryEntry:
        """Read a public dotted name's parameters, summary, own class members and rule links.

        The name is a class or function that find returns, or a method or property of a class,
        such as pandas.DataFrame.merge. parameters_written holds each parameter as the source
        writes it, with its type annotation and default. A stub file writes ... for a default,
        which says only that the parameter is optional.

        Args:
            name: A dotted name, such as pandas.DataFrame or pandas.DataFrame.merge.
        """
        entries = await xpd.run(API, Path.cwd(), [name])
        return entries[0]

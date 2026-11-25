"""Expose structural search, rewrites, outlines and rule authoring through xpd."""

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult

from xpd.mcp.base import READS, REWRITES, WRITES, confirmed, rooted, run
from xpd.mcp.binary import NEW, OUTLINE, QUERY, REWRITE, TEST, Xpd
from xpd.mcp.models import CaseReport, CodeMatch, CreatedFiles, OutlineFile
from xpd.mcp.names import ToolName
from xpd.mcp.options import CaseOptions, NewItem, OutlineOptions, Query
from xpd.mcp.root import Project


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    register_queries(mcp, xpd, project)

    @mcp.tool(name=ToolName.OUTLINE, tags={"structure"}, annotations=READS, task=True)
    @rooted
    async def outline(
        ctx: Context, paths: list[str] | None = None, options: OutlineOptions | None = None
    ) -> list[OutlineFile] | InputRequiredResult:
        """Read symbols, signatures, imports, members and source ranges without execution.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
            options: The language and items to read, and an extra outline rule file.
        """
        root = await project.required_root(ctx)
        options = options or OutlineOptions()
        flags = options.flags(root)
        return await run(ctx, xpd, OUTLINE, root, root.all_inside(paths or ["."]), *flags)

    register_authoring(mcp, xpd, project)


def register_queries(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(name=ToolName.QUERY, tags={"structure"}, annotations=READS, task=True)
    @rooted
    async def query(
        ctx: Context, query: Query, paths: list[str] | None = None
    ) -> list[CodeMatch] | InputRequiredResult:
        """Find AST patterns or syntax kinds and preview replacements without changing files.

        Args:
            query: The pattern or kind to find, and the replacement to preview.
            paths: Files or folders inside the project root. Default: the whole root.
        """
        root = await project.required_root(ctx)
        return await run(ctx, xpd, QUERY, root, root.all_inside(paths or ["."]), *query.flags())

    @mcp.tool(name=ToolName.REWRITE, tags={"structure", "fix"}, annotations=REWRITES, task=True)
    @rooted
    async def rewrite(
        ctx: Context, query: Query, paths: list[str] | None = None
    ) -> list[CodeMatch] | InputRequiredResult:
        """Preview a structural rewrite, ask for confirmation, then apply it to project files.

        Args:
            query: The pattern or kind to find, and the `rewrite` template that replaces it.
            paths: Files or folders inside the project root. Default: the whole root.
        """
        if query.rewrite is None:
            msg = "Provide a rewrite template."
            raise ToolError(msg)
        root = await project.required_root(ctx)
        paths = root.all_inside(paths or ["."])
        flags = query.flags()
        preview = await run(ctx, xpd, QUERY, root, paths, *flags)
        if not preview:
            return preview
        answer = await confirmed(
            ctx,
            project,
            f"Apply {len(preview)} structural rewrites? They may change the code's behavior.",
            key="confirm_unsafe_fixes",
            title="Apply unsafe fixes",
        )
        if isinstance(answer, InputRequiredResult):
            return answer
        if not answer:
            return preview
        await run(ctx, xpd, REWRITE, root, paths, *flags)
        return preview


def register_authoring(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(name=ToolName.NEW, tags={"rules", "authoring"}, annotations=WRITES, task=True)
    @rooted
    async def new(ctx: Context, item: NewItem) -> CreatedFiles | InputRequiredResult:
        """Create a project, rule, empty case template or shared utility without prompts.

        Args:
            item: What to create, and the answers that `xpd rules new` would otherwise ask for.
        """
        root = await project.required_root(ctx)
        names = [item.name] if item.name is not None else []
        return await run(ctx, xpd, NEW, root, names, *item.flags(root))

    @mcp.tool(name=ToolName.TEST, tags={"rules", "authoring"}, annotations=WRITES, task=True)
    @rooted
    async def test_rules(
        ctx: Context, options: CaseOptions | None = None
    ) -> CaseReport | InputRequiredResult:
        """Check valid and invalid examples using xpd's scanner and compare snapshots.

        Changes snapshots only when update_snapshots is true. Import, version and proof
        checks use the same rules as linting, including project rules.

        Args:
            options: The project file, the test folder and filter, and what to do with snapshots.
        """
        root = await project.required_root(ctx)
        return await run(ctx, xpd, TEST, root, [], *(options or CaseOptions()).flags(root))

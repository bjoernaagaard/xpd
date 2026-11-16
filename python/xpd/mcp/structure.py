"""Expose structural search, rewrites, outlines and rule authoring through xpd."""

from typing import Literal

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult
from pydantic import BaseModel, Field

from xpd.mcp.binary import NEW, OUTLINE, QUERY, REWRITE, TEST, Xpd
from xpd.mcp.models import CaseReport, CodeMatch, CreatedFiles, FixSafety, OutlineFile, Severity
from xpd.mcp.root import Project, Root
from xpd.mcp.tools import READS, WRITES, confirmed, run


class Query(BaseModel):
    pattern: str | None = Field(default=None, description="AST pattern with $VARIABLE captures.")
    kind: str | None = Field(default=None, description="Syntax kind instead of a pattern.")
    language: str | None = None
    selector: str | None = None
    rewrite: str | None = Field(default=None, description="Replacement template to preview.")

    def flags(self) -> list[str]:
        if (self.pattern is None) == (self.kind is None):
            msg = "Provide exactly one of pattern or kind."
            raise ToolError(msg)
        flags: list[str] = []
        for option, value in (
            ("--pattern", self.pattern),
            ("--kind", self.kind),
            ("--lang", self.language),
            ("--selector", self.selector),
            ("--rewrite", self.rewrite),
        ):
            if value is not None:
                flags.extend((option, value))
        return flags


class OutlineOptions(BaseModel):
    language: str | None = None
    items: Literal["all", "structure", "imports", "exports"] = "all"
    rules: str | None = Field(default=None, description="Project-relative outline rule file.")


class NewItem(BaseModel):
    entity: Literal["project", "rule", "test", "util"]
    name: str | None = None
    directory: str = "."
    language: str | None = None
    pack: str | None = None
    pattern: str | None = None
    message: str | None = None
    severity: Severity | None = None
    note: str | None = None
    rewrite: str | None = None
    fix_safety: FixSafety | None = None
    since: str | None = None
    until: str | None = None
    holds: list[str] = Field(default_factory=list, description="Proofs as VAR={kind: call}.")
    unless_holds: list[str] = Field(default_factory=list)
    unless_imported: str | None = None
    public_id: str | None = None
    with_test: bool = False
    every_file: bool = False

    def flags(self, root: Root) -> list[str]:
        flags = [self.entity]
        if self.name is not None:
            flags.append(self.name)
        flags.extend(("--yes", "--format", "json", "--directory", root.inside(self.directory)))
        for option, value in (
            ("--lang", self.language),
            ("--pack", self.pack),
            ("--pattern", self.pattern),
            ("--message", self.message),
            ("--severity", self.severity),
            ("--note", self.note),
            ("--fix", self.rewrite),
            ("--fix-safety", self.fix_safety),
            ("--since", self.since),
            ("--until", self.until),
            ("--unless-imported", self.unless_imported),
            ("--public-id", self.public_id),
        ):
            if value is not None:
                flags.extend((option, value))
        for option, values in (("--holds", self.holds), ("--unless-holds", self.unless_holds)):
            for value in values:
                flags.extend((option, value))
        if self.with_test:
            flags.append("--with-test")
        if self.every_file:
            flags.append("--every-file")
        return flags


class CaseOptions(BaseModel):
    config: str | None = None
    test_dir: str | None = None
    filter: str | None = None
    skip_snapshots: bool = False
    update_snapshots: bool = False

    def flags(self, root: Root) -> list[str]:
        if self.skip_snapshots and self.update_snapshots:
            msg = "Choose either skipping or updating snapshots."
            raise ToolError(msg)
        flags: list[str] = []
        for option, path in (("--config", self.config), ("--test-dir", self.test_dir)):
            if path is not None:
                flags.extend((option, root.inside(path)))
        if self.filter is not None:
            flags.extend(("--filter", self.filter))
        if self.skip_snapshots:
            flags.append("--skip-snapshot-tests")
        if self.update_snapshots:
            flags.append("--update-all")
        return flags


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    register_queries(mcp, xpd, project)
    register_outline(mcp, xpd, project)
    register_authoring(mcp, xpd, project)


def register_queries(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"structure"}, annotations=READS, task=True)
    async def query(
        ctx: Context, query: Query, paths: list[str] | None = None
    ) -> list[CodeMatch] | InputRequiredResult:
        """Find AST patterns or syntax kinds and preview replacements without changing files."""
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        return await run(ctx, xpd, QUERY, root, root.all_inside(paths or ["."]), *query.flags())

    @mcp.tool(tags={"structure", "fix"}, annotations=WRITES, task=True)
    async def rewrite(
        ctx: Context, query: Query, paths: list[str] | None = None
    ) -> list[CodeMatch] | InputRequiredResult:
        """Preview a structural rewrite, ask for confirmation, then apply it to project files."""
        if query.rewrite is None:
            msg = "Provide a rewrite template."
            raise ToolError(msg)
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        paths = root.all_inside(paths or ["."])
        flags = query.flags()
        preview = await run(ctx, xpd, QUERY, root, paths, *flags)
        if not preview:
            return preview
        answer = await confirmed(
            ctx, f"Apply {len(preview)} structural rewrites? They may change the code's behavior."
        )
        if isinstance(answer, InputRequiredResult):
            requests = {**(answer.input_requests or {}), **project.roots_request(ctx)}
            return answer.model_copy(update={"input_requests": requests})
        if not answer:
            return preview
        output = await run(ctx, xpd, REWRITE, root, paths, *flags)
        await ctx.info(output)
        return preview


def register_outline(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"structure"}, annotations=READS, task=True)
    async def outline(
        ctx: Context, paths: list[str] | None = None, options: OutlineOptions | None = None
    ) -> list[OutlineFile] | InputRequiredResult:
        """Read symbols, signatures, imports, members and source ranges without execution."""
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        options = options or OutlineOptions()
        flags = ["--items", options.items]
        if options.language is not None:
            flags.extend(("--lang", options.language))
        if options.rules is not None:
            flags.extend(("--outline-rules", root.inside(options.rules)))
        return await run(ctx, xpd, OUTLINE, root, root.all_inside(paths or ["."]), *flags)


def register_authoring(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"rules", "authoring"}, annotations=WRITES, task=True)
    async def new(ctx: Context, item: NewItem) -> CreatedFiles | InputRequiredResult:
        """Create a project, rule, empty case template or shared utility without prompts."""
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        return await run(ctx, xpd, NEW, root, [], *item.flags(root))

    @mcp.tool(name="test", tags={"rules", "authoring"}, annotations=WRITES, task=True)
    async def test_rules(
        ctx: Context, options: CaseOptions | None = None
    ) -> CaseReport | InputRequiredResult:
        """Check valid and invalid examples using xpd's scanner and compare snapshots.

        Changes snapshots only when update_snapshots is true. Import, version and proof
        checks use the same rules as linting, including project rules.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        return await run(ctx, xpd, TEST, root, [], *(options or CaseOptions()).flags(root))

"""Expose structural search, rewrites, outlines and rule authoring through xpd."""

from typing import Literal

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult
from pydantic import BaseModel, Field

from xpd.mcp.base import READS, REWRITES, WRITES, confirmed, rooted, run
from xpd.mcp.binary import NEW, OUTLINE, QUERY, REWRITE, TEST, Xpd
from xpd.mcp.models import CaseReport, CodeMatch, CreatedFiles, FixSafety, OutlineFile, Severity
from xpd.mcp.root import Project, Root


class Query(BaseModel):
    pattern: str | None = Field(default=None, description="AST pattern with $VARIABLE captures.")
    kind: str | None = Field(default=None, description="Syntax kind instead of a pattern.")
    language: str | None = Field(
        default=None,
        description="The language of the pattern, such as python. Default: each file's extension.",
    )
    selector: str | None = Field(
        default=None, description="The kind of the part of the pattern that is the match."
    )
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
                flags.append(f"{option}={value}")
        return flags


class OutlineOptions(BaseModel):
    language: str | None = Field(
        default=None, description="Read only the files of this language, such as python."
    )
    items: Literal["auto", "all", "structure", "imports", "exports"] = Field(
        default="auto",
        description="Which top-level items to list. `auto` is `structure` for files and "
        "`exports` for folders.",
    )
    rules: str | None = Field(default=None, description="Project-relative outline rule file.")


class NewItem(BaseModel):
    entity: Literal["project", "rule", "test", "util"] = Field(description="What to create.")
    name: str | None = Field(
        default=None,
        description="The rule's name or <pack>/<name>, the test's rule id, or the utility's name.",
    )
    directory: str = Field(
        default=".",
        description="The project folder, which holds sgconfig.yml, relative to the project root.",
    )
    language: str | None = Field(
        default=None, description="The language of a structural rule or a utility."
    )
    pack: str | None = Field(
        default=None,
        description="The pack the rule belongs to, or the pack a new project declares.",
    )
    pattern: str | None = Field(
        default=None,
        description="The code to find, with $X for any one node and $$$X for any number of them.",
    )
    message: str | None = Field(
        default=None, description="What the issue says. It may name the pattern's variables."
    )
    severity: Severity | None = Field(default=None, description="Default: warning.")
    note: str | None = Field(
        default=None, description="A longer explanation, for the rule's note and its Markdown."
    )
    rewrite: str | None = Field(
        default=None, description="The code that replaces a match, with the pattern's variables."
    )
    fix_safety: FixSafety | None = Field(
        default=None,
        description="Whether the fix keeps the code's behavior. Required with rewrite.",
    )
    since: str | None = Field(
        default=None, description="The first version of the pack's library the rule applies to."
    )
    until: str | None = Field(
        default=None,
        description="The first version of the pack's library the rule no longer applies to.",
    )
    holds: list[str] = Field(default_factory=list, description="Proofs as VAR={kind: call}.")
    unless_holds: list[str] = Field(
        default_factory=list,
        description="Drop a match when the variable holds a value of this shape, as "
        "VAR={kind: call}.",
    )
    unless_imported: str | None = Field(
        default=None,
        description="Silence the rule in a file that imports a module with a module-level "
        "statement of this shape.",
    )
    public_id: str | None = Field(
        default=None, description="The id that issues show, for one of several rule variants."
    )
    with_test: bool = Field(default=False, description="Also create an empty test case.")
    every_file: bool = Field(
        default=False,
        description="Run a pack this call adds on every Python file, not only on files that "
        "import its module.",
    )

    def flags(self, root: Root) -> list[str]:
        flags = [self.entity]
        flags.extend(("--yes", "--format=json", f"--directory={root.inside(self.directory)}"))
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
                flags.append(f"{option}={value}")
        for option, values in (("--holds", self.holds), ("--unless-holds", self.unless_holds)):
            flags.extend(f"{option}={value}" for value in values)
        if self.with_test:
            flags.append("--with-test")
        if self.every_file:
            flags.append("--every-file")
        return flags


class CaseOptions(BaseModel):
    config: str | None = Field(
        default=None,
        description="A project file to read instead of the nearest sgconfig.yml, relative to the "
        "project root.",
    )
    test_dir: str | None = Field(
        default=None,
        description="Read the test cases in this folder instead of the testConfigs of "
        "sgconfig.yml.",
    )
    filter: str | None = Field(
        default=None, description="Test only the rules whose id matches this regular expression."
    )
    skip_snapshots: bool = Field(
        default=False, description="Check only the valid and invalid examples, and no snapshot."
    )
    update_snapshots: bool = Field(
        default=False, description="Accept every changed snapshot and write it."
    )

    def flags(self, root: Root) -> list[str]:
        if self.skip_snapshots and self.update_snapshots:
            msg = "Choose either skipping or updating snapshots."
            raise ToolError(msg)
        flags: list[str] = []
        for option, path in (("--config", self.config), ("--test-dir", self.test_dir)):
            if path is not None:
                flags.append(f"{option}={root.inside(path)}")
        if self.filter is not None:
            flags.append(f"--filter={self.filter}")
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

    @mcp.tool(tags={"structure", "fix"}, annotations=REWRITES, task=True)
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
        output = await run(ctx, xpd, REWRITE, root, paths, *flags)
        await ctx.info(output)
        return preview


def register_outline(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"structure"}, annotations=READS, task=True)
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
        flags = [f"--items={options.items}"]
        if options.language is not None:
            flags.append(f"--lang={options.language}")
        if options.rules is not None:
            flags.append(f"--outline-rules={root.inside(options.rules)}")
        return await run(ctx, xpd, OUTLINE, root, root.all_inside(paths or ["."]), *flags)


def register_authoring(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(tags={"rules", "authoring"}, annotations=WRITES, task=True)
    @rooted
    async def new(ctx: Context, item: NewItem) -> CreatedFiles | InputRequiredResult:
        """Create a project, rule, empty case template or shared utility without prompts.

        Args:
            item: What to create, and the answers that `xpd new` would otherwise ask for.
        """
        root = await project.required_root(ctx)
        names = [item.name] if item.name is not None else []
        return await run(ctx, xpd, NEW, root, names, *item.flags(root))

    @mcp.tool(name="test", tags={"rules", "authoring"}, annotations=WRITES, task=True)
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

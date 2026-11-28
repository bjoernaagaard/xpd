"""Builds the xpd MCP server from its settings, and runs it."""

from collections.abc import Sequence

from fastmcp import FastMCP
from fastmcp.server.auth import StaticTokenVerifier
from fastmcp.skills import Skill
from fastmcp.skills.provider import SkillCatalogProvider
from fastmcp_tasks import TasksExtension

from xpd.mcp import animation, catalog, discovery, pipeline, prompts, views
from xpd.mcp.binary import BUILTIN_RULES, SKILLS, Xpd, find_binary
from xpd.mcp.names import ToolName
from xpd.mcp.root import Project, Root
from xpd.mcp.settings import Settings
from xpd.mcp.tools import Analysis

HTTP_RESTRICTED_TOOLS = frozenset({ToolName.GATES})


def instructions(skill_uris: dict[str, str], *, local: bool) -> str:
    animate = (
        f"- When asked to animate a concept as a video, call `{ToolName.ANIMATE_GUIDE}` and "
        f"`{ToolName.ANIMATE_CATALOG}` to compare sibling Manim primitives before coding. "
        f"Use `{ToolName.ANIMATE_STATUS}` to check the renderer, "
        f"`{ToolName.ANIMATE_RENDER}` with `draft` or `preview` to iterate, and "
        f"`{ToolName.ANIMATE_ARTIFACT}` to inspect the contact sheet before the final render.\n"
        if local
        else ""
    )
    return f"""\
xpd lints Python for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn, scikit-lego, \
MAPIE, Category Encoders, linearmodels, arch, pymoo, plotly and Apache Hamilton, reads the \
dataflow of step code and Hamilton modules, profiles data files, and checks that a refactor keeps \
its result. Its commands run in your shell, and this server adds what a command cannot do. \
Every path is relative to the project root.

- Before choosing a library, run `xpd find "<task>"`, then `xpd api <name>` with a returned \
name. `xpd api Class.method` prints a method's parameters with their defaults.
- After you write or change Python, run `xpd check <paths>`. Fix every error. Fix a warning \
unless you can say why the code is right. `xpd check --fix` writes the safe fixes, and \
`xpd api <pack>/<rule>` explains a rule.
- When you make, change or review a chart, in plotly or any other library, run \
`xpd chart-review <file>` and answer each question of Tufte's book that it prints.
- For model feature engineering, run `xpd feature-review` and follow \
{skill_uris["xpd-feature-review"]}.
- After you add, move or rename a step or a Hamilton node, run `xpd graph <file>`, and reuse \
columns that exist instead of computing their formulas again.
- Before you choose methods for a data file, run `xpd profile <file>`.
- After you refactor data logic, run `xpd equiv <script.py> --since HEAD`. The refactor is done \
when it prints `"equivalent": true`.
- Before and after a refactor of a codebase, run `xpd untangle` (or call `show_map` and \
`show_ledger` for visual coupling and workflow summaries).
- To build a model from a data file, follow the `build_pipeline` prompt. Call \
`{ToolName.DISCOVERY_GATE}` on the files once the library is chosen, and the run is done when \
`{ToolName.GATES}` returns `passed: true`. `{ToolName.GATES}` runs the stages itself.
{animate}- The show_ tools draw what `xpd check`, `xpd graph` and `xpd profile` find as tables \
and charts in hosts that render apps.
- {skill_uris["xpd"]}, {skill_uris["python-best-practices"]} and \
{skill_uris["python-review"]} hold the style a linter cannot check."""


def build(settings: Settings) -> FastMCP:
    transport, _ = settings.validate_transport()
    auth = (
        StaticTokenVerifier(
            tokens={settings.auth_token: {"client_id": "xpd-client", "scopes": ["xpd"]}}
        )
        if transport != "stdio" and settings.auth_token
        else None
    )
    project = Project(Root.at(settings.root) if settings.root else None)
    xpd = Xpd(find_binary())
    fallback_rules = catalog.Catalog(tuple(xpd.run_now(BUILTIN_RULES)))
    skills = []
    for skill in xpd.run_now(SKILLS):
        files = {file.path: file.text for file in skill.files}
        skills.append(Skill.from_files(files))
    skill_uris = {skill.name: str(skill.to_mcp_skill().uri) for skill in skills}
    mcp = FastMCP(
        "xpd",
        instructions=instructions(skill_uris, local=transport == "stdio"),
        version=xpd.version(),
        website_url="https://bjoernaagaard.github.io/xpd/",
        auth=auth,
        providers=[SkillCatalogProvider(skills)],
    )
    mcp.add_extension(TasksExtension())
    analysis = Analysis(xpd, project)
    discovery.register(mcp, xpd, project)
    views.register(mcp, analysis)
    catalog.register(mcp, fallback_rules, xpd, project)
    pipeline.register(mcp, analysis)
    prompts.register(mcp, skill_uris["python-review"])
    animation.register(mcp, xpd, project, local=transport == "stdio")

    @mcp.resource(
        "xpd://version",
        name="version",
        mime_type="text/plain",
        annotations={"audience": ["user", "assistant"], "priority": 0.2},
    )
    def version() -> str:
        """The version of the xpd binary that this server runs."""
        return mcp.version or ""

    if transport != "stdio" and not settings.allow_writes:
        mcp.disable(names=set(HTTP_RESTRICTED_TOOLS), components={"tool"})
    return mcp


def main(argv: Sequence[str] | None = None) -> None:
    settings = Settings.from_args(argv)
    try:
        transport, _ = settings.validate_transport()
    except ValueError as error:
        raise SystemExit(str(error)) from error
    mcp = build(settings)
    options = settings.transport_options() if transport != "stdio" else {}
    mcp.run(transport=transport, **options)

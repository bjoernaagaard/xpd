"""Builds the xpd MCP server from its settings, and runs it."""

from collections.abc import Sequence

from fastmcp import FastMCP
from fastmcp.experimental.transforms.code_mode import CodeMode
from fastmcp.server.auth import StaticTokenVerifier
from fastmcp.skills import Skill
from fastmcp.skills.provider import SkillCatalogProvider
from fastmcp_tasks import TasksExtension

from xpd.mcp import animation, catalog, discovery, pipeline, prompts, structure, tools, views
from xpd.mcp.binary import BUILTIN_RULES, SKILLS, Xpd, find_binary
from xpd.mcp.names import ToolName
from xpd.mcp.root import Project, Root
from xpd.mcp.settings import Settings

HTTP_RESTRICTED_TOOLS = frozenset(
    {
        ToolName.FIX,
        ToolName.REWRITE,
        ToolName.NEW,
        ToolName.TEST,
        ToolName.REFIT,
        ToolName.EQUIV,
    }
)


def instructions(skill_uris: dict[str, str], *, local: bool) -> str:
    animate = (
        f"- When asked to animate a concept as a video, call `{ToolName.ANIMATE_GUIDE}`. "
        f"Use `{ToolName.ANIMATE_STATUS}` to check the renderer, "
        f"`{ToolName.ANIMATE_RENDER}` to preview the scene, and "
        f"`{ToolName.ANIMATE_ARTIFACT}` to inspect the contact sheet before the final render.\n"
        if local
        else ""
    )
    return f"""\
xpd lints Python for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn, scikit-lego, \
MAPIE, Category Encoders, linearmodels, arch, pymoo, plotly and Apache Hamilton, reads the \
dataflow of step code and Hamilton modules, profiles data files, and checks that a refactor keeps \
its result. Every path is relative to the project root.

- Before choosing a library, call `{ToolName.FIND}` with the task, then `{ToolName.API}` with \
a returned name. Call \
`{ToolName.API}` with `Class.method` to read a method's parameters with their defaults before \
you call it.\

- After choosing and writing code, call `{ToolName.DISCOVERY_GATE}` on its files before \
`{ToolName.EQUIV}`.\

- After you write or change Python, call `{ToolName.CHECK}`. Fix every error. Fix a warning \
unless you can \
say why the code is right. `{ToolName.FIX}` writes xpd's fixes, and asks the user before \
unsafe ones.
- When you make, change or review a chart, in plotly or any other library, call \
`{ToolName.CHART_REVIEW}` on its file. Answer each question of Tufte's book that it returns, and \
draw a chart of another library with its plotly route.
- Use `{ToolName.QUERY}` to find code patterns and preview replacements, and \
`{ToolName.REWRITE}` to apply them after \
confirmation. Use `{ToolName.OUTLINE}` to browse project symbols, imports and members in \
supported languages.
- Use `{ToolName.NEW}` to create a local rule project, rule, empty case template or shared utility.
- To learn why a rule fires and see its before and after code, call `{ToolName.RULE}`. \
`{ToolName.RULES}` lists and \
filters all of them.
- After you add, move or rename a step or a Hamilton node, call `{ToolName.GRAPH}`, and reuse \
columns that \
exist instead of computing their formulas again.
- Before you choose methods for a data file, call `{ToolName.PROFILE}`.
- After you refactor data logic, call `{ToolName.EQUIV}` on the old and new script. The \
refactor is done \
when it says `equivalent: true`.
- To build a model from a data file, follow the `build_pipeline` prompt. The run is done when \
`{ToolName.GATES}` returns `passed: true` for the results of `{ToolName.PROFILE}`, \
`{ToolName.CHECK}`, `{ToolName.GRAPH}`, `{ToolName.EQUIV}`, \
`{ToolName.REFIT}` and `{ToolName.CONTRACT}`. With code mode on, one `execute` call chains them.
{animate}- The show_ tools draw the same results as tables and charts in hosts that render apps.
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
    skills = [
        Skill.from_files({file.path: file.text for file in skill.files})
        for skill in xpd.run_now(SKILLS)
    ]
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
    analysis = tools.register(mcp, xpd, project)
    structure.register(mcp, xpd, project)
    discovery.register(mcp, xpd, project)
    views.register(mcp, analysis)
    catalog.register(mcp, fallback_rules, xpd, project)
    pipeline.register(mcp, project, fallback_rules)
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
    if settings.code_mode:
        mcp.add_transform(CodeMode())

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

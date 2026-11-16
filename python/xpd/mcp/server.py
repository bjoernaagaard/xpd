"""Builds the xpd MCP server from its settings, and runs it."""

from collections.abc import Sequence

import fastmcp
from fastmcp import FastMCP
from fastmcp.experimental.transforms.code_mode import CodeMode
from fastmcp.skills import Skill
from fastmcp.skills.provider import SkillCatalogProvider
from fastmcp_tasks import TasksExtension

from xpd.mcp import catalog, discovery, pipeline, prompts, structure, tools, views
from xpd.mcp.binary import BUILTIN_RULES, SKILLS, Xpd, find_binary
from xpd.mcp.root import Project, Root
from xpd.mcp.settings import Settings

INSTRUCTIONS = """\
xpd lints Python for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn, scikit-lego, \
MAPIE, Category Encoders, linearmodels, arch, pymoo, plotly and Apache Hamilton, reads the \
dataflow of step code and Hamilton modules, profiles data files, and checks that a refactor keeps \
its result. Every path is relative to the project root.

- Before choosing a library, call `find` with the task, then `api` with a returned name.\

- After choosing and writing code, call `discovery_gate` on its files before `equiv`.\

- After you write or change Python, call `check`. Fix every error. Fix a warning unless you can \
say why the code is right. `fix` writes xpd's fixes, and asks the user before unsafe ones.
- Use `query` to find code patterns and preview replacements, and `rewrite` to apply them after \
confirmation. Use `outline` to browse project symbols, imports and members in supported languages.
- Use `new` to create a local rule project, rule, empty case template or shared utility.
- To learn why a rule fires and see its before and after code, call `rule`. `rules` lists and \
filters all of them.
- After you add, move or rename a step or a Hamilton node, call `graph`, and reuse columns that \
exist instead of computing their formulas again.
- Before you choose methods for a data file, call `profile`.
- After you refactor data logic, call `equiv` on the old and new script. The refactor is done \
when it says `equivalent: true`.
- To build a model from a data file, follow the `build_pipeline` prompt. The run is done when \
`gates` returns `passed: true` for the results of `profile`, `check`, `graph`, `equiv`, \
`refit` and `contract`. With code mode on, one `execute` call chains them.
- The show_ tools draw the same results as tables and charts in hosts that render apps.
- skill://xpd/SKILL.md, skill://python-best-practices/SKILL.md and \
skill://python-review/SKILL.md hold the style a linter cannot check."""


def build(settings: Settings) -> FastMCP:
    project = Project(Root.at(settings.root) if settings.root else None)
    xpd = Xpd(find_binary())
    fallback_rules = catalog.Catalog(tuple(xpd.run_now(BUILTIN_RULES)))
    skills = [
        Skill.from_files({file.path: file.text for file in skill.files})
        for skill in xpd.run_now(SKILLS)
    ]
    mcp = FastMCP(
        "xpd",
        instructions=INSTRUCTIONS,
        version=xpd.version(),
        website_url="https://bjoernaagaard.github.io/xpd/",
        providers=[SkillCatalogProvider(skills)],
        transforms=[CodeMode()] if settings.code_mode else [],
    )
    mcp.add_extension(TasksExtension())
    tools.register(mcp, xpd, project)
    structure.register(mcp, xpd, project)
    discovery.register(mcp, xpd, project)
    views.register(mcp, xpd, project)
    catalog.register(mcp, fallback_rules, xpd, project)
    pipeline.register(mcp, project, fallback_rules)
    prompts.register(mcp)

    @mcp.resource(
        "xpd://version",
        name="version",
        mime_type="text/plain",
        annotations={"audience": ["user", "assistant"], "priority": 0.2},
    )
    def version() -> str:
        """The version of the xpd binary that this server runs."""
        return mcp.version or ""

    return mcp


def main(argv: Sequence[str] | None = None) -> None:
    settings = Settings.from_args(argv)
    mcp = build(settings)
    transport = settings.transport or fastmcp.settings.transport
    options = settings.transport_options() if transport != "stdio" else {}
    mcp.run(transport=transport, **options)

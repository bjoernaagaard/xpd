"""Builds the xpd MCP server from its settings, and runs it."""

from collections.abc import Sequence

import fastmcp
from fastmcp import FastMCP
from fastmcp.experimental.transforms.code_mode import CodeMode
from fastmcp.skills import Skill
from fastmcp.skills.provider import SkillCatalogProvider
from fastmcp_tasks import TasksExtension

from xpd.mcp import catalog, prompts, tools, views
from xpd.mcp.binary import RULES, SKILLS, Xpd, find_binary
from xpd.mcp.root import Project, Root
from xpd.mcp.settings import Settings

INSTRUCTIONS = """\
xpd lints Python for slow or wrong habits with pandas, NumPy, SciPy, scikit-learn, scikit-lego, \
MAPIE, Category Encoders and Apache Hamilton, reads the dataflow of step code and Hamilton \
modules, profiles data files, and checks that a refactor keeps its result. Every path is relative \
to the project root.

- After you write or change Python, call `check`. Fix every error. Fix a warning unless you can \
say why the code is right. `fix` writes xpd's fixes, and asks the user before unsafe ones.
- To learn why a rule fires and see its before and after code, call `rule`. `rules` lists and \
filters all of them.
- After you add, move or rename a step or a Hamilton node, call `graph`, and reuse columns that \
exist instead of computing their formulas again.
- Before you choose methods for a data file, call `profile`.
- After you refactor data logic, call `equiv` on the old and new script. The refactor is done \
when it says `equivalent: true`.
- The show_ tools draw the same results as tables and charts in hosts that render apps.
- skill://xpd/SKILL.md, skill://python-best-practices/SKILL.md and \
skill://python-review/SKILL.md hold the style a linter cannot check."""


def build(settings: Settings) -> FastMCP:
    project = Project(Root.at(settings.root) if settings.root else None)
    xpd = Xpd(find_binary())
    # The binary's rules and skills do not change while it runs, so the server reads them once.
    rules = catalog.Catalog(tuple(xpd.run_now(RULES)))
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
    views.register(mcp, xpd, project)
    catalog.register(mcp, rules)
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

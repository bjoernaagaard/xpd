"""What the server runs with the xpd binary on the project's files. The commands themselves are
no tools, because an agent runs them in its shell. The show tools and `gates` call them here."""

from typing import NamedTuple

from fastmcp import Context
from mcp.types import InputRequiredResult

from xpd.mcp.base import rooted, run
from xpd.mcp.binary import CHECK, GRAPH, PROFILE, Xpd
from xpd.mcp.models import CheckReport, GraphReport, Profile
from xpd.mcp.root import Project


class Analysis(NamedTuple):
    xpd: Xpd
    project: Project

    @rooted
    async def check(
        self, ctx: Context, paths: list[str] | None = None
    ) -> CheckReport | InputRequiredResult:
        """What `xpd check` reports for the paths, or for the whole root."""
        root = await self.project.required_root(ctx)
        return await run(ctx, self.xpd, CHECK, root, root.all_inside(paths or ["."]))

    @rooted
    async def graph(self, ctx: Context, paths: list[str]) -> GraphReport | InputRequiredResult:
        """What `xpd graph` reads in the paths."""
        root = await self.project.required_root(ctx)
        return await run(ctx, self.xpd, GRAPH, root, root.all_inside(paths))

    @rooted
    async def profile(self, ctx: Context, file: str) -> Profile | InputRequiredResult:
        """What `xpd profile` prints for the data file."""
        root = await self.project.required_root(ctx)
        return await run(ctx, self.xpd, PROFILE, root, [root.inside(file)])

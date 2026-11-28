"""What the server runs with the xpd binary on the project's files. The commands themselves are
no tools, because an agent runs them in its shell. The show tools and `gates` call them here."""

from typing import NamedTuple

from fastmcp import Context
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult

from xpd.mcp.base import rooted, run
from xpd.mcp.binary import (
    CHECK,
    GRAPH,
    PROFILE,
    UNTANGLE_MAP,
    Xpd,
    ledger_status_command,
    ledger_survey_command,
)
from xpd.mcp.models import (
    CheckReport,
    CouplingMapReport,
    GraphReport,
    LedgerViewReport,
    LedgerWorkflowName,
    Profile,
)
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

    @rooted
    async def coupling_map(
        self, ctx: Context, paths: list[str] | None = None
    ) -> CouplingMapReport | InputRequiredResult:
        """What `xpd untangle --map` reads for the paths, or for the whole root."""
        root = await self.project.required_root(ctx)
        return await run(ctx, self.xpd, UNTANGLE_MAP, root, root.all_inside(paths or ["."]))

    @rooted
    async def ledger_view(
        self,
        ctx: Context,
        workflow: LedgerWorkflowName,
        paths: list[str] | None = None,
    ) -> LedgerViewReport | InputRequiredResult:
        """Active durable run status for `workflow`, falling back to a read-only survey."""
        root = await self.project.required_root(ctx)
        try:
            payload = await self.xpd.run(ledger_status_command(workflow), root.path, ())
            return LedgerViewReport(workflow=workflow, mode="run", payload=payload)
        except ToolError:
            payload = await run(
                ctx,
                self.xpd,
                ledger_survey_command(workflow),
                root,
                root.all_inside(paths or ["."]),
            )
            return LedgerViewReport(workflow=workflow, mode="survey", payload=payload)

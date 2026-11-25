"""App tools: the results of check, graph and profile as UIs that a host renders with Prefab.

An app tool's structured content is the UI. A view's text content is the same JSON that the
command prints, for a host that renders no UI and for the model."""

from collections import Counter
from itertools import count
from typing import get_args

from fastmcp import Context, FastMCP
from fastmcp.tools import ToolResult
from mcp.types import InputRequiredResult
from prefab_ui.app import PrefabApp
from prefab_ui.components import (
    Badge,
    Column,
    DataTable,
    DataTableColumn,
    Heading,
    Mermaid,
    Metric,
    Row,
)

from xpd.mcp.base import READS
from xpd.mcp.models import GraphReport, Issue, Output, Problem, Severity
from xpd.mcp.tools import Analysis

SEVERITY_BADGES = {
    "error": "destructive",
    "warning": "warning",
    "info": "info",
    "hint": "secondary",
    "note": "secondary",
}


def columns(*keys: tuple[str, str]) -> list[DataTableColumn]:
    return [DataTableColumn(key=key, header=header, sortable=True) for key, header in keys]


def shown(view: PrefabApp, data: Output) -> ToolResult:
    return ToolResult(content=data.model_dump_json(), structured_content=view)


def label(text: str) -> str:
    """`text` as a quoted Mermaid label. Mermaid reads `#quot;` as a double quote."""
    return '"' + text.replace('"', "#quot;") + '"'


def dataflow_chart(report: GraphReport) -> str:
    """A Mermaid flowchart with one box per step or node and one arrow per read. A box's id is
    its place in the chart, such as s0 or n3, since a step of a chain is named like
    `.assign(gross)` and a name such as `end` is a Mermaid keyword. Steps are named within
    their pipeline, and Apache Hamilton nodes and inputs within the whole report."""
    ids = count()

    def box(names: dict[str, str], prefix: str, name: str) -> str:
        if name not in names:
            names[name] = f"{prefix}{next(ids)}"
        return names[name]

    lines = ["flowchart LR"]
    for pipeline in report.pipelines:
        steps: dict[str, str] = {}
        for step in pipeline.steps:
            shown = f"{step.name}()" if step.kind == "function" else step.name
            lines.append(f"  {box(steps, 's', step.name)}[{label(shown)}]")
        for link in pipeline.links:
            source, to = box(steps, "s", link.source), box(steps, "s", link.to)
            lines.append(f"  {source} -->|{label(', '.join(link.columns))}| {to}")
    nodes: dict[str, str] = {}
    lines.extend(f"  {box(nodes, 'n', name.name)}([{label(name.name)}])" for name in report.inputs)
    for node in report.nodes:
        to = box(nodes, "n", node.name)
        lines.append(f"  {to}[{label(node.name)}]")
        lines.extend(f"  {box(nodes, 'n', source.name)} --> {to}" for source in node.inputs)
    return "\n".join(lines)


def issue_row(issue: Issue) -> dict[str, object]:
    return {
        "place": f"{issue.file}:{issue.line}" if issue.line else issue.file,
        "severity": Badge(issue.severity, variant=SEVERITY_BADGES[issue.severity]),
        "rule": f"{issue.package}/{issue.rule}",
        "fix": issue.fix or "",
        "message": issue.message,
    }


def problem_row(problem: Problem) -> dict[str, object]:
    return {
        "place": f"{problem.file}:{problem.line}",
        "severity": Badge(problem.severity, variant=SEVERITY_BADGES[problem.severity]),
        "message": problem.message,
    }


def register(mcp: FastMCP, analysis: Analysis) -> None:
    @mcp.tool(app=True, tags={"lint", "app"}, annotations=READS)
    async def show_issues(
        ctx: Context, paths: list[str] | None = None
    ) -> ToolResult | InputRequiredResult:
        """Show what `check` finds as a sortable, searchable table, in hosts that render MCP
        Apps. Run `xpd check` when you need the data.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
        """
        report = await analysis.check(ctx, paths)
        if isinstance(report, InputRequiredResult):
            return report
        rows = [issue_row(issue) for issue in report.issues]
        counts = Counter(issue.severity for issue in report.issues)
        with PrefabApp() as app, Column(gap=4, css_class="p-6"):
            with Row(gap=6):
                for level in get_args(Severity):
                    if level == "hint":
                        continue
                    Metric(label=f"{level}s", value=counts[level])
                Metric(label="fixable", value=report.fixable)
            DataTable(
                columns=columns(
                    ("place", "Place"),
                    ("severity", "Severity"),
                    ("rule", "Rule"),
                    ("fix", "Fix"),
                    ("message", "Message"),
                ),
                rows=rows,
                search=True,
                paginated=True,
            )
        return shown(app, report)

    @mcp.tool(app=True, tags={"dataflow", "app"}, annotations=READS)
    async def show_dataflow(ctx: Context, paths: list[str]) -> ToolResult | InputRequiredResult:
        """Show what `graph` reads as a flowchart of steps or nodes and a table of its problems,
        in hosts that render MCP Apps. Run `xpd graph` when you need the data.

        Args:
            paths: Files or folders inside the project root.
        """
        report = await analysis.graph(ctx, paths)
        if isinstance(report, InputRequiredResult):
            return report
        rows = [problem_row(problem) for problem in report.problems]
        with PrefabApp() as app, Column(gap=4, css_class="p-6"):
            Heading(content="Dataflow")
            Mermaid(chart=dataflow_chart(report))
            DataTable(
                columns=columns(
                    ("place", "Place"), ("severity", "Severity"), ("message", "Message")
                ),
                rows=rows,
            )
        return shown(app, report)

    @mcp.tool(app=True, tags={"data", "app"}, annotations=READS)
    async def show_columns(ctx: Context, file: str) -> ToolResult | InputRequiredResult:
        """Show what `profile` reads as a table of columns, in hosts that render MCP Apps. Use
        `profile` when you need the data. Needs pandas in the project's Python.

        Args:
            file: The data file, inside the project root.
        """
        profile = await analysis.profile(ctx, file)
        if isinstance(profile, InputRequiredResult):
            return profile
        rows = [column.model_dump() for column in profile.columns]
        with PrefabApp() as app, Column(gap=4, css_class="p-6"):
            with Row(gap=6):
                Metric(label="rows", value=profile.rows)
                Metric(label="columns", value=len(profile.columns))
                Metric(label="memory", value=f"{profile.memory_bytes:,} bytes")
            DataTable(
                columns=columns(
                    ("column", "Column"),
                    ("dtype", "dtype"),
                    ("nulls", "Nulls"),
                    ("unique", "Unique"),
                    ("memory_bytes", "Bytes"),
                ),
                rows=rows,
                search=True,
            )
        return shown(app, profile)

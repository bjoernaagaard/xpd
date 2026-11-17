"""App tools: the results of check, graph and profile as UIs that a host renders with Prefab.

An app tool's structured content is the UI, so each view is a tool of its own and the data tools
keep their output schemas. A view's text content is the same JSON the data tool returns, for a
host that renders no UI and for the model."""

from collections import Counter

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

from xpd.mcp.binary import CHECK, GRAPH, PROFILE, Xpd
from xpd.mcp.models import GraphReport, Output
from xpd.mcp.root import Project
from xpd.mcp.tools import READS, run

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
    boxes: dict[tuple[int, str], str] = {}

    def box(pipeline: int, name: str) -> str:
        prefix = "n" if pipeline < 0 else "s"
        return boxes.setdefault((pipeline, name), f"{prefix}{len(boxes)}")

    lines = ["flowchart LR"]
    for index, pipeline in enumerate(report.pipelines):
        for step in pipeline.steps:
            shown = f"{step.name}()" if step.kind == "function" else step.name
            lines.append(f"  {box(index, step.name)}[{label(shown)}]")
        lines.extend(
            f"  {box(index, link.source)} -->|{label(', '.join(link.columns))}| "
            f"{box(index, link.to)}"
            for link in pipeline.links
        )
    lines.extend(f"  {box(-1, name.name)}([{label(name.name)}])" for name in report.inputs)
    for node in report.nodes:
        lines.append(f"  {box(-1, node.name)}[{label(node.name)}]")
        lines.extend(f"  {box(-1, source.name)} --> {box(-1, node.name)}" for source in node.inputs)
    return "\n".join(lines)


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(app=True, tags={"lint", "app"}, annotations=READS)
    async def show_issues(
        ctx: Context, paths: list[str] | None = None
    ) -> ToolResult | InputRequiredResult:
        """Show what `check` finds as a sortable, searchable table, in hosts that render MCP
        Apps. Use `check` when you need the data.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        report = await run(ctx, xpd, CHECK, root, root.all_inside(paths or ["."]))
        rows = [
            {
                "place": f"{issue.file}:{issue.line}" if issue.line else issue.file,
                "severity": Badge(issue.severity, variant=SEVERITY_BADGES[issue.severity]),
                "rule": f"{issue.package}/{issue.rule}",
                "fix": issue.fix or "",
                "message": issue.message,
            }
            for issue in report.issues
        ]
        counts = Counter(issue.severity for issue in report.issues)
        with PrefabApp() as app, Column(gap=4, css_class="p-6"):
            with Row(gap=6):
                for level in ("error", "warning", "info"):
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
        in hosts that render MCP Apps. Use `graph` when you need the data.

        Args:
            paths: Files or folders inside the project root.
        """
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        report = await run(ctx, xpd, GRAPH, root, root.all_inside(paths))
        rows = [
            {
                "place": f"{problem.file}:{problem.line}",
                "severity": Badge(problem.severity, variant=SEVERITY_BADGES[problem.severity]),
                "message": problem.message,
            }
            for problem in report.problems
        ]
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
        root = await project.root(ctx)
        if isinstance(root, InputRequiredResult):
            return root
        profile = await run(ctx, xpd, PROFILE, root, [root.inside(file)])
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
                rows=[column.model_dump() for column in profile.columns],
                search=True,
            )
        return shown(app, profile)

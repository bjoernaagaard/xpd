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
from xpd.mcp.models import (
    CouplingMapReport,
    GraphReport,
    Issue,
    LedgerWorkflowName,
    Output,
    Problem,
    Severity,
)
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


ROLE_BADGES = {
    "cycle": "destructive",
    "mixed": "warning",
    "orphan": "secondary",
    "facade": "info",
    "single-use": "secondary",
    "normal": "outline",
}


def coupling_chart(report: CouplingMapReport) -> str:
    lines = ["flowchart LR"]
    for node in report.nodes:
        badge = f", {node.role}" if node.role != "normal" else ""
        lines.append(f"  {node.id}[{label(f'{node.path} ({node.loc} LOC{badge})')}]")
    lines.extend(
        f"  {edge.source} {'-->' if edge.eager else '-.->'} {edge.to}" for edge in report.edges
    )
    lines.extend(f"  {link.first} -.- {link.second}" for link in report.hidden_links)
    return "\n".join(lines)


def _run_rows(run_data: dict[str, object]) -> tuple[int, int, list[dict[str, object]]]:
    accepted = len(run_data["accepted"]) if isinstance(run_data.get("accepted"), dict) else 0
    rounds = run_data.get("rounds")
    fixed = (
        sum(
            len(entry["fixed"])
            for entry in rounds
            if isinstance(entry, dict) and isinstance(entry.get("fixed"), list)
        )
        if isinstance(rounds, list)
        else 0
    )
    answered = run_data.get("answered")
    rows: list[dict[str, object]] = []
    if isinstance(answered, dict):
        for item_id, record in answered.items():
            verdict = record.get("verdict", "") if isinstance(record, dict) else ""
            reason = record.get("reason", "") if isinstance(record, dict) else ""
            rows.append({"item": item_id, "status": str(verdict), "detail": str(reason)})
    return fixed, accepted, rows


def _finding_row(finding: dict[str, object]) -> dict[str, object]:
    places = finding.get("places")
    first_place = ""
    if isinstance(places, list) and places and isinstance(places[0], dict):
        file_name = str(places[0].get("path", ""))
        line_num = places[0].get("line")
        first_place = f"{file_name}:{line_num}" if line_num else file_name
    return {
        "item": first_place or str(finding.get("id", "")),
        "status": str(finding.get("kind", "")),
        "detail": str(finding.get("evidence", "")),
    }


def ledger_summary(
    mode: str, payload: dict[str, object]
) -> tuple[str, int, int, list[dict[str, object]]]:
    if mode == "run" and isinstance(payload.get("run"), dict):
        next_data = payload.get("next")
        stage = (
            next_data["stage"]
            if isinstance(next_data, dict) and isinstance(next_data.get("stage"), str)
            else "run"
        )
        fixed, accepted, rows = _run_rows(payload["run"])
        return stage, fixed, accepted, rows
    findings = payload.get("findings")
    rows = (
        [_finding_row(item) for item in findings if isinstance(item, dict)]
        if isinstance(findings, list)
        else []
    )
    return "survey", 0, 0, rows


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

    register_ledger_views(mcp, analysis)


def register_ledger_views(mcp: FastMCP, analysis: Analysis) -> None:
    @mcp.tool(app=True, tags={"untangle", "app"}, annotations=READS)
    async def show_map(
        ctx: Context, paths: list[str] | None = None
    ) -> ToolResult | InputRequiredResult:
        """Show the module coupling and health map (`xpd untangle --map`) as a Mermaid diagram
        and sortable hotspot table, in hosts that render MCP Apps.

        Args:
            paths: Files or folders inside the project root. Default: the whole root.
        """
        report = await analysis.coupling_map(ctx, paths)
        if isinstance(report, InputRequiredResult):
            return report
        counts = Counter(node.role for node in report.nodes)
        rows = [
            {
                "path": node.path,
                "package": node.package,
                "loc": node.loc,
                "role": Badge(node.role, variant=ROLE_BADGES.get(node.role, "secondary")),
            }
            for node in report.nodes
        ]
        with PrefabApp() as app, Column(gap=4, css_class="p-6"):
            Heading(content="Module Coupling Map")
            with Row(gap=6):
                Metric(label="modules", value=report.total_modules)
                Metric(label="edges", value=len(report.edges))
                Metric(label="cycles", value=counts["cycle"])
                Metric(label="orphans", value=counts["orphan"])
                Metric(label="mixed", value=counts["mixed"])
                Metric(label="hidden links", value=len(report.hidden_links))
            Mermaid(chart=coupling_chart(report))
            DataTable(
                columns=columns(
                    ("path", "Module"),
                    ("package", "Package"),
                    ("loc", "LOC"),
                    ("role", "Health"),
                ),
                rows=rows,
                search=True,
                paginated=True,
            )
        return shown(app, report)

    @mcp.tool(app=True, tags={"ledger", "app"}, annotations=READS)
    async def show_ledger(
        ctx: Context,
        workflow: LedgerWorkflowName,
        paths: list[str] | None = None,
    ) -> ToolResult | InputRequiredResult:
        """Show live Duroxide run progress or a read-only survey for any registered xpd ledger
        workflow (`untangle`, `optimize`, `harden`, `discover`, `data-prep`, `ts-review`,
        `feature-review`, `model-review`, `opt-review`, or `chart-review`).

        Args:
            workflow: Registered xpd workflow CLI name.
            paths: Files or folders inside the project root when no recorded run is active.
        """
        report = await analysis.ledger_view(ctx, workflow, paths)
        if isinstance(report, InputRequiredResult):
            return report
        stage, fixed, accepted, rows = ledger_summary(report.mode, report.payload)
        with PrefabApp() as app, Column(gap=4, css_class="p-6"):
            Heading(content=f"xpd {workflow}")
            with Row(gap=6):
                Metric(label="mode", value=report.mode)
                Metric(label="stage", value=stage)
                Metric(label="items", value=len(rows))
                Metric(label="fixed", value=fixed)
                Metric(label="accepted", value=accepted)
            DataTable(
                columns=columns(
                    ("item", "Item / Place"),
                    ("status", "Kind / Verdict"),
                    ("detail", "Detail"),
                ),
                rows=rows,
                search=True,
                paginated=True,
            )
        return shown(app, report)

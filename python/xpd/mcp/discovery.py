"""Check direct imported calls against the binary's source index without importing libraries."""

import ast
from pathlib import Path

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from mcp.types import InputRequiredResult

from xpd.mcp.base import READS, rooted
from xpd.mcp.binary import API, Xpd
from xpd.mcp.models import Discovery, DiscoveryEntry, DiscoveryProof, Gate
from xpd.mcp.names import ToolName
from xpd.mcp.root import Project


def dotted(node: ast.expr) -> str | None:
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute) and (parent := dotted(node.value)):
        return f"{parent}.{node.attr}"
    return None


def imports(tree: ast.Module) -> dict[str, str]:
    names = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                names[alias.asname or alias.name.split(".")[0]] = (
                    alias.name if alias.asname else alias.name.split(".")[0]
                )
        elif isinstance(node, ast.ImportFrom) and node.module and not node.level:
            for alias in node.names:
                names[alias.asname or alias.name] = f"{node.module}.{alias.name}"
    return names


def keyword_problems(node: ast.Call, entry: DiscoveryEntry, place: str) -> list[str]:
    failures = []
    for keyword in node.keywords:
        if keyword.arg is None:
            failures.append(f"{place} {entry.name} has ** arguments whose names are unseen.")
        elif keyword.arg not in entry.parameters:
            failures.append(f"{place} {entry.name} has no indexed parameter {keyword.arg}.")
    return failures


def choice_problems(discovery: Discovery) -> list[str]:
    choice = discovery.choice
    failures = []
    if choice.name not in {entry.name for entry in discovery.candidates.entries()}:
        failures.append(f"{choice.name} is not among the recorded candidates.")
    selected = next(
        (entry for entry in discovery.candidates.entries() if entry.name == choice.name), None
    )
    if selected is not None and selected.kind == "symbol" and choice.name not in choice.symbols:
        failures.append("The chosen symbol must be listed among the symbols the code calls.")
    return failures


async def inspect_calls(xpd: Xpd, root: Path, paths: list[str], discovery: Discovery) -> Gate:
    choice = discovery.choice
    failures = choice_problems(discovery)
    modules = {name.rsplit(".", 1)[0] for name in choice.symbols}
    used = set()
    checked = 0
    for path in paths:
        try:
            tree = ast.parse((root / path).read_text(encoding="utf-8"), filename=path)
        except (OSError, SyntaxError, UnicodeError) as error:
            failures.append(f"Cannot parse {path}. {error}")
            continue
        names = imports(tree)
        for node in ast.walk(tree):
            if not isinstance(node, ast.Call) or not (name := dotted(node.func)):
                continue
            head, *tail = name.split(".")
            if head not in names:
                continue
            name = ".".join([names[head], *tail])
            if name.rsplit(".", 1)[0] not in modules:
                continue
            place = f"{path}:{node.lineno}"
            try:
                entries = await xpd.run(API, root, [name])
            except ToolError:
                failures.append(f"{place} {name} is not known to api.")
                continue
            entry = next((item for item in entries if item.name == name), None)
            if entry is None or entry.signature is None:
                failures.append(f"{place} {name} is not an indexed callable symbol.")
                continue
            used.add(name)
            checked += 1
            failures.extend(keyword_problems(node, entry, place))
    failures.extend(
        f"No direct imported call to {name}." for name in choice.symbols if name not in used
    )
    return Gate(
        name="indexed-api-calls",
        stage="discovery",
        evidence=["api", "source"],
        passed=not failures,
        detail=" ".join(failures)
        or (
            f"Checked {checked} direct imported calls for indexed callable names and literal "
            "keyword names in api's parameter lists. No code was run."
        ),
    )


def register(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(name=ToolName.DISCOVERY_GATE, tags={"discovery", "pipeline"}, annotations=READS)
    @rooted
    async def discovery_gate(
        ctx: Context, paths: list[str], discovery: Discovery
    ) -> DiscoveryProof | InputRequiredResult:
        """Read Python files and check direct imported calls in the chosen symbols' modules.

        Resolve import aliases and ask the binary's api for each direct call's dotted name.
        Require every chosen symbol to be called and every literal keyword to occur in the
        index's parameter list. Fail on unknown names and ** arguments. Run before equiv.
        This is a source spelling check, not Python name or type resolution. It cannot see
        instance methods, runtime aliases, shadowing, argument values, positional binding,
        required arguments, or dependency availability. The index may omit valid APIs and
        inherited or variadic parameters. It runs no project code and imports no library.
        Pass its result as proof.discovery, and save discovery in the training contract.

        Args:
            paths: The Python files or folders to check, inside the project root.
            discovery: The search, the entries read and the choice, as the agent recorded them.
        """
        root = await project.required_root(ctx)
        resolved = root.all_inside(paths)
        result = await inspect_calls(xpd, root.path, resolved, discovery)
        return DiscoveryProof(discovery=discovery, paths=resolved, gate=result)

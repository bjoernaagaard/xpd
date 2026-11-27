import ast
import csv
import json
import runpy
import sys
from pathlib import Path

from plotly.offline import get_plotlyjs, get_plotlyjs_version

MAX_TABLE_ROWS = 50
METADATA_ASSIGNMENT_ERROR = "The metadata section must assign one literal EXAMPLE"
SPLIT_ERROR = "An example needs one top-level make_before, make_after and EXAMPLE, and a preamble"


def _section_name(node: ast.stmt) -> str:
    match node:
        case ast.FunctionDef(name="make_before" | "make_after" as name):
            return name.removeprefix("make_")
        case ast.Assign(targets=[ast.Name(id="EXAMPLE")]):
            return "metadata"
        case _:
            return "preamble"


def split_example(source: Path) -> None:
    lines = source.read_text(encoding="utf-8").splitlines(keepends=True)
    sections: dict[str, list[str]] = {"preamble": [], "before": [], "after": [], "metadata": []}
    counts = dict.fromkeys(sections, 0)
    for node in ast.parse("".join(lines)).body:
        name = _section_name(node)
        counts[name] += 1
        first = min([node.lineno, *(item.lineno for item in getattr(node, "decorator_list", []))])
        sections[name].append("".join(lines[first - 1 : node.end_lineno]))
    if counts["preamble"] == 0 or any(
        counts[name] != 1 for name in ("before", "after", "metadata")
    ):
        raise ValueError(SPLIT_ERROR)
    order = ("preamble", "before", "after", "metadata")
    sys.stdout.write(json.dumps(["\n\n".join(sections[name]) for name in order]))


def inspect_example(preamble: Path, metadata: Path, folder: Path) -> None:
    statements = ast.parse(metadata.read_text(encoding="utf-8").strip()).body
    match statements:
        case [ast.Assign(targets=[ast.Name(id="EXAMPLE")], value=value)]:
            example = ast.literal_eval(value)
        case _:
            raise ValueError(METADATA_ASSIGNMENT_ERROR)
    functions = runpy.run_path(str(preamble))
    table = functions["value_table"](functions["load_data"]())
    if table is not None:
        columns, rows = table["columns"], table["rows"]
        if (
            not isinstance(columns, list)
            or not columns
            or not all(isinstance(column, str) and column.strip() for column in columns)
        ):
            msg = "Value table needs a list of nonempty column headers"
            raise ValueError(msg)
        if (
            not isinstance(rows, list)
            or not rows
            or any(
                not isinstance(row, list)
                or len(row) != len(columns)
                or any(cell is not None and type(cell) not in (str, int, float) for cell in row)
                for row in rows
            )
        ):
            msg = "Value table needs rows with one text, numeric or null cell per column"
            raise ValueError(msg)
        if len(rows) > MAX_TABLE_ROWS:
            with (folder / "values.csv").open("w", encoding="utf-8", newline="") as file:
                writer = csv.writer(file)
                writer.writerow(columns)
                writer.writerows(rows)
            table = None
    sys.stdout.write(json.dumps({"example": example, "table": table}, allow_nan=False))


def adapter_payload(figure: Path, extractor: Path, variant: str) -> None:
    decode_bdata = runpy.run_path(str(extractor))["decode_bdata"]
    spec = decode_bdata(json.loads(figure.read_text(encoding="utf-8")))
    if not spec.get("data"):
        msg = "Plotly figure must have at least one trace"
        raise ValueError(msg)
    sys.stdout.write(
        json.dumps({"figures": [{"name": variant, "library": "plotly", "spec": spec}]})
    )


def main() -> None:
    mode, *args = sys.argv[1:]
    if mode == "split":
        split_example(Path(args[0]))
    elif mode == "metadata":
        inspect_example(*map(Path, args))
    elif mode == "adapter":
        adapter_payload(Path(args[0]), Path(args[1]), args[2])
    elif mode == "bundle":
        version = get_plotlyjs_version()
        (Path(args[0]) / f"plotly-{version}.min.js").write_text(get_plotlyjs(), encoding="utf-8")
        sys.stdout.write(version)
    else:
        msg = f"Unknown Tufte example operation: {mode}"
        raise ValueError(msg)


if __name__ == "__main__":
    main()

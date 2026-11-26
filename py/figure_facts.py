"""Extract rendered figure and table specs from a Python adapter for `xpd chart-review --adapter`.

The adapter is a Python file defining `figures() -> dict[str, object]`. Supported objects are
Plotly `go.Figure` (charts and `go.Table`), Dash AG Grid `AgGrid` components (or plain dicts with
`columnDefs` and `rowData`), and Matplotlib `Figure`, `Axes`, or Seaborn grids. Plotly figures
are resolved through `full_figure_for_development` when Kaleido is available, falling back to
`to_dict()`. Matplotlib figures are drawn with the Agg backend. The script prints one JSON object
with `figures`, or `error` with exit code 2.
"""

from __future__ import annotations

import base64
import importlib
import json
import os
import runpy
import sys
from contextlib import redirect_stdout
from pathlib import Path
from typing import NoReturn

PAIR = 2


def fail(error: str) -> NoReturn:
    """Print an error payload and exit with status 2."""
    print(json.dumps({"figures": [], "error": error}))
    sys.exit(2)


def decode_bdata(value: object) -> object:
    """Recursively expand Plotly base64-encoded typed arrays into plain JSON lists."""
    if isinstance(value, dict):
        if "bdata" in value and "dtype" in value:
            numpy = importlib.import_module("numpy")
            raw = base64.b64decode(value["bdata"])
            return numpy.frombuffer(raw, dtype=value["dtype"]).tolist()
        return {k: decode_bdata(v) for k, v in value.items()}
    if isinstance(value, list):
        return [decode_bdata(item) for item in value]
    if hasattr(value, "tolist"):
        return value.tolist()
    return value


def plotly_entry(name: str, figure: object) -> dict:
    """Resolve a Plotly Figure (chart or go.Table) into a normalized JSON spec."""
    try:
        spec = figure.full_figure_for_development(warn=False, as_dict=True)
        unresolved = None
    except Exception as error:  # noqa: BLE001
        spec = figure.to_dict()
        unresolved = f"{type(error).__name__}: {error}".splitlines()[0]
    return {
        "name": name,
        "library": "plotly",
        "unresolved": unresolved,
        "spec": decode_bdata(spec),
    }


def ag_grid_props(figure: object) -> dict | None:
    """Extract AG Grid props from a Dash AgGrid component or a plain dict spec."""
    if isinstance(figure, dict):
        if "columnDefs" in figure or "rowData" in figure:
            return figure
        inner = figure.get("spec")
        if isinstance(inner, dict) and ("columnDefs" in inner or "rowData" in inner):
            return inner
        return None
    if hasattr(figure, "to_plotly_json"):
        payload = figure.to_plotly_json()
        if isinstance(payload, dict):
            props = payload.get("props")
            if isinstance(props, dict) and ("columnDefs" in props or "rowData" in props):
                return props
    return None


def matplotlib_figure(figure: object) -> object:
    """Return the underlying Matplotlib Figure for a Figure, Axes, or Seaborn grid."""
    for attribute in ("figure", "fig"):
        inner = getattr(figure, attribute, None)
        if inner is not None and hasattr(inner, "savefig") and inner is not figure:
            return inner
    return figure


def hex_color(color: tuple) -> str:
    """Convert an RGB(A) tuple in 0..1 to a hex color string."""
    red, green, blue = (round(part * 255) for part in color[:3])
    return f"#{red:02x}{green:02x}{blue:02x}"


def number(value: object) -> object:
    """Unwrap a NumPy scalar when present."""
    return value.item() if hasattr(value, "item") else value


def series_name(label: str, kind: str, at: int) -> str:
    """Return a visible series label or a positional fallback."""
    return f"{kind} {at}" if not label or label.startswith("_") else label


def line_marks(ax: object, *, dated: bool, found: dict) -> None:
    """Collect visible Matplotlib line traces from an Axes."""
    dates = importlib.import_module("matplotlib.dates")
    for line in ax.lines:
        x = [number(value) for value in line.get_xdata()]
        y = [number(value) for value in line.get_ydata()]
        if not line.get_visible() or len({str(value) for value in x}) < PAIR:
            continue
        name = series_name(line.get_label(), "line", len(found["marks"]))
        if dated:
            x = [dates.num2date(value).date().isoformat() for value in x]
        color = line.get_color()
        found["marks"].append({"trace": name, "mark": "lines", "values": len(x)})
        found["styles"].append(
            {
                "trace": name,
                "color": color if isinstance(color, str) else hex_color(color),
                "dash": line.get_linestyle(),
                "markers": line.get_marker() not in {None, "None", "", " "},
                "direct_labels": False,
                "width": float(line.get_linewidth()),
            }
        )
        found["xs"].append({"trace": name, "x": x, "y": y})


def bar_marks(ax: object, found: dict) -> None:
    """Collect visible Matplotlib bar containers from an Axes."""
    bars_of = importlib.import_module("matplotlib.container").BarContainer
    ticks = [tick.get_text() for tick in ax.get_xticklabels() if tick.get_text()]
    for container in ax.containers:
        if not isinstance(container, bars_of):
            continue
        bars = [patch for patch in container.patches if patch.get_visible()]
        name = series_name(container.get_label(), "bars", len(found["marks"]))
        colors = {hex_color(patch.get_facecolor()) for patch in bars}
        found["marks"].append({"trace": name, "mark": "bar", "values": len(bars)})
        found["styles"].append(
            {
                "trace": name,
                "color": next(iter(colors)) if len(colors) == 1 else None,
                "dash": None,
                "markers": False,
                "direct_labels": False,
                "width": None,
            }
        )
        centers = [number(patch.get_x() + patch.get_width() / 2) for patch in bars]
        heights = [number(patch.get_height()) for patch in bars]
        found["xs"].append({"trace": name, "x": ticks or centers, "y": heights})


def other_marks(ax: object, found: dict) -> None:
    """Collect Matplotlib pie wedges, scatter collections, area polygons, and images."""
    wedge = importlib.import_module("matplotlib.patches").Wedge
    collections = importlib.import_module("matplotlib.collections")
    wedges = [patch for patch in ax.patches if isinstance(patch, wedge) and patch.get_visible()]
    if wedges:
        found["marks"].append({"trace": "pie", "mark": "pie", "values": len(wedges)})
    for collection in ax.collections:
        if not collection.get_visible():
            continue
        if isinstance(collection, collections.PathCollection):
            count, mark = len(collection.get_offsets()), "scatter"
        elif isinstance(collection, collections.PolyCollection):
            count, mark = len(collection.get_paths()), "area"
        else:
            continue
        if count:
            name = series_name(collection.get_label(), mark, len(found["marks"]))
            found["marks"].append({"trace": name, "mark": mark, "values": count})
    for image in ax.images:
        size = image.get_array().size
        found["marks"].append({"trace": "image", "mark": "heatmap", "values": size})


def matplotlib_texts(figure: object, axes: list) -> dict:
    """Extract titles, axis titles, tick rotations, and annotations from a Matplotlib Figure."""
    suptitle = getattr(figure, "get_suptitle", str)()
    titles = [suptitle] if suptitle else []
    axis_titles, angles, notes = [], set(), []
    for ax in axes:
        if ax.get_title():
            titles.append(ax.get_title())
        axis_titles += [label for label in (ax.get_xlabel(), ax.get_ylabel()) if label]
        angles |= {round(tick.get_rotation()) for tick in ax.get_xticklabels() if tick.get_text()}
        notes += [text.get_text() for text in ax.texts if text.get_visible() and text.get_text()]
    return {
        "title": " / ".join(titles),
        "axis_titles": axis_titles,
        "tick_angles": sorted(angles),
        "annotations": notes,
    }


def matplotlib_entry(name: str, figure: object) -> dict:
    """Render a Matplotlib or Seaborn figure with Agg and return its raw measurements."""
    dates = importlib.import_module("matplotlib.dates")
    date_formats = (dates.AutoDateFormatter, dates.ConciseDateFormatter, dates.DateFormatter)
    figure = matplotlib_figure(figure)
    figure.canvas.draw()
    width, height = (round(side) for side in figure.get_size_inches() * figure.dpi)
    axes = [ax for ax in figure.axes if ax.get_visible() and ax.get_label() != "<colorbar>"]
    found = {"marks": [], "styles": [], "xs": []}
    y_ranges = []
    has_box_spines = False
    for ax in axes:
        dated = isinstance(ax.xaxis.get_major_formatter(), date_formats)
        line_marks(ax, dated=dated, found=found)
        bar_marks(ax, found)
        other_marks(ax, found)
        y_ranges.append([float(v) for v in ax.get_ylim()])
        if all(spine.get_visible() for spine in ax.spines.values()):
            has_box_spines = True
    texts = matplotlib_texts(figure, axes)
    for style in found["styles"]:
        style["direct_labels"] = bool(texts["annotations"])
    positions = [tuple(round(v, 3) for v in ax.get_position().bounds) for ax in axes]
    has_twin = len(positions) != len(set(positions))
    return {
        "name": name,
        "library": "matplotlib",
        "raw": {
            "canvas": {"width": width, "height": height, "panels": len(set(positions)) or 1},
            "dpi": float(figure.dpi),
            **found,
            "legend": any(ax.get_legend() is not None for ax in axes) or bool(figure.legends),
            "texts": texts,
            "y_ranges": y_ranges,
            "has_twin": has_twin,
            "has_box_spines": has_box_spines,
        },
    }


def extract_figure(name: str, figure: object) -> dict:
    """Dispatch one figure returned by `figures()` into its normalized JSON representation."""
    with redirect_stdout(sys.stderr):
        if hasattr(figure, "full_figure_for_development"):
            return plotly_entry(name, figure)
        grid = ag_grid_props(figure)
        if grid is not None:
            return {"name": name, "library": "ag-grid", "spec": decode_bdata(grid)}
        if isinstance(figure, dict) and ("data" in figure or "layout" in figure):
            return {
                "name": name,
                "library": "plotly",
                "unresolved": None,
                "spec": decode_bdata(figure),
            }
        if hasattr(matplotlib_figure(figure), "savefig"):
            return matplotlib_entry(name, figure)
        kind = type(figure).__name__
        fail(f"{name} is a {kind}, which is no Plotly, AG Grid, or Matplotlib figure")


def adapter_figures(adapter: Path) -> dict:
    """Load `adapter` and return the dict produced by its `figures()` function."""
    try:
        with redirect_stdout(sys.stderr):
            namespace = runpy.run_path(str(adapter), run_name="__xpd_chart_review__")
            make = namespace.get("figures")
            if not callable(make):
                fail(f"{adapter} defines no figures() that returns a dict of name to figure")
            figures = make()
    except Exception as error:  # noqa: BLE001
        fail(f"{adapter} raised {type(error).__name__}: {error}")
    if not isinstance(figures, dict) or not figures:
        fail(f"figures() in {adapter} returns no dict of name to figure")
    return figures


def main(adapter: Path) -> None:
    """Run `adapter` and print the normalized figures JSON to stdout."""
    os.environ.setdefault("MPLBACKEND", "Agg")
    sys.path.insert(0, str(adapter.resolve().parent))
    sys.path.insert(0, str(Path.cwd()))
    extracted = []
    for name, figure in adapter_figures(adapter).items():
        try:
            extracted.append(extract_figure(str(name), figure))
        except Exception as error:  # noqa: BLE001
            fail(f"cannot measure {name}: {type(error).__name__}: {error}")
    print(json.dumps({"figures": extracted}, default=str))


if __name__ == "__main__":
    arguments = sys.argv[1:]
    if len(arguments) != 1:
        fail("usage: figure_facts.py <adapter.py>")
    main(Path(arguments[0]))

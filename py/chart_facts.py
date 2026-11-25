"""Measure the facts of the figures that a chart adapter hands over, for `xpd chart-review --run`.

The adapter is a Python file that defines `figures()`, which returns a dict of name to figure: a
plotly `go.Figure`, a matplotlib `Figure`, or a seaborn grid or matplotlib `Axes`, whose figure
is read. For a Dash app the adapter imports the app and calls its callback with named inputs.
Plotly figures are resolved by `full_figure_for_development`, which draws them with Kaleido and
Chrome. Matplotlib and seaborn figures are drawn with the Agg backend, which needs no browser.
The script prints one JSON object with `figures`, or `error` and exit code 2.

A plotly canvas is measured in CSS pixels, 96 to the inch. A figure with 20 values or fewer gets
a finding, because a table or a sentence can show them. A pure red has a red channel of at least
0.6 and other channels of at most 0.35, and a pure green has a green channel of at least 0.4 and
other channels of at most 0.35; a reader who cannot tell red from green cannot tell the two
apart. Text that sits on matplotlib axes counts as a direct label, as a label at the end of a
line does. Agg is chosen before matplotlib is imported, because it draws without a display.
"""

from __future__ import annotations

import base64
import importlib
import json
import os
import re
import runpy
import sys
from contextlib import redirect_stdout
from datetime import date, datetime
from pathlib import Path
from typing import NoReturn

CSS_DPI = 96
SMALL_DATA = 20
PAIR = 2
MONEY = re.compile(
    r"(€|\$|£|¥|\beur\b|\beuros?\b|\busd\b|\bdollars?\b|\bdkk\b|\bkroner\b|\bgbp\b"
    r"|\bprice|\bcost|\brevenue|\bincome|\bwage|\bsalar)",
    re.IGNORECASE,
)
BASIS = re.compile(
    r"\b(nominal|real|constant|deflated|inflation|adjusted|current prices"
    r"|\d{4} (euro|dollar|prices))",
    re.IGNORECASE,
)
YEARS = range(1800, 2201)
DOMAIN_TRACES = {"pie", "sunburst", "treemap", "funnelarea", "indicator"}
STRONG_RED = 0.6
STRONG_GREEN = 0.4
WEAK = 0.35


def fail(error: str) -> NoReturn:
    print(json.dumps({"figures": [], "error": error}))
    sys.exit(2)


def sequence(value: object) -> list:
    """The values of a plotly data array, which a figure may hold as base64 typed data."""
    if value is None:
        return []
    if isinstance(value, dict) and "bdata" in value:
        numpy = importlib.import_module("numpy")
        return numpy.frombuffer(base64.b64decode(value["bdata"]), dtype=value["dtype"]).tolist()
    if isinstance(value, (str, bytes)):
        return [value]
    try:
        return list(value)
    except TypeError:
        return [value]


def year_of(value: object) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (datetime, date)):
        return value.year
    if isinstance(value, (int, float)) and float(value).is_integer() and int(value) in YEARS:
        return int(value)
    if isinstance(value, str):
        found = re.match(r"^(\d{4})(-\d{2}(-\d{2})?)?([ T].*)?$", value.strip())
        if found and int(found.group(1)) in YEARS:
            return int(found.group(1))
    return None


def rgb(color: object) -> tuple[float, float, float] | None:
    """A color as red, green and blue from 0 to 1, or None when it is no single color."""
    if not isinstance(color, str):
        return None
    found = re.fullmatch(r"\s*rgba?\(([^)]*)\)\s*", color)
    if found:
        red, green, blue = (float(part) / 255 for part in found.group(1).split(",")[:3])
        return (red, green, blue)
    try:
        colors = importlib.import_module("matplotlib.colors")
        return tuple(colors.to_rgb(color))
    except (ImportError, ValueError):
        return None


def hue(color: object) -> str | None:
    """`red` or `green` for a color that a red-green deficient reader cannot tell apart."""
    value = rgb(color)
    if value is None:
        return None
    red, green, blue = value
    if red >= STRONG_RED and green <= WEAK and blue <= WEAK:
        return "red"
    if green >= STRONG_GREEN and red <= WEAK and blue <= WEAK:
        return "green"
    return None


def capitals(text: str) -> bool:
    words = [word for word in re.findall(r"[^\W\d_]+", text) if len(word) > 1]
    return len(words) >= PAIR and all(word.isupper() for word in words)


def plain(text: object) -> str:
    """Text without HTML tags, and without the placeholder that plotly.js gives an unset title."""
    if not isinstance(text, str) or text.startswith("Click to enter"):
        return ""
    return re.sub(r"<[^>]+>", "", text)


def series_name(label: str, kind: str, at: int) -> str:
    """A matplotlib label, or a name by position for a label that matplotlib keeps hidden."""
    return f"{kind} {at}" if not label or label.startswith("_") else label


def plotly_figure(figure: object) -> tuple[dict, str | None]:
    """The figure as plotly.js draws it, or as written with the reason it is not resolved."""
    try:
        return figure.full_figure_for_development(warn=False, as_dict=True), None
    except Exception as error:  # noqa: BLE001
        return figure.to_dict(), f"{type(error).__name__}: {error}".splitlines()[0]


def plotly_count(trace: dict) -> int:
    kind = trace.get("type", "scatter")
    if kind in DOMAIN_TRACES:
        return len(sequence(trace.get("values") or trace.get("labels")))
    if kind == "bar":
        return len(sequence(trace.get("x" if trace.get("orientation") == "h" else "y")))
    if kind in {"heatmap", "contour", "surface"}:
        return sum(len(sequence(row)) for row in sequence(trace.get("z")))
    return len(sequence(trace.get("x"))) or len(sequence(trace.get("y")))


def plotly_style(name: str, trace: dict) -> dict:
    mode = trace.get("mode", "")
    lines = "lines" in mode
    line, marker = trace.get("line") or {}, trace.get("marker") or {}
    color = line.get("color") if lines else marker.get("color")
    return {
        "trace": name,
        "color": color if isinstance(color, str) else None,
        "dash": line.get("dash") if lines else None,
        "markers": "markers" in mode,
        "direct_labels": "text" in mode or bool(trace.get("texttemplate")),
    }


def plotly_texts(layout: dict) -> dict:
    axes = sorted(key for key in layout if re.fullmatch(r"[xy]axis\d*", key))
    titles = [plain(((layout[key] or {}).get("title") or {}).get("text")) for key in axes]
    angles = {(layout[key] or {}).get("tickangle") for key in axes}
    notes = [plain(note.get("text")) for note in layout.get("annotations", [])]
    return {
        "title": plain((layout.get("title") or {}).get("text")),
        "axis_titles": [title for title in titles if title],
        "tick_angles": sorted(angle for angle in angles if isinstance(angle, (int, float))),
        "annotations": [note for note in notes if note],
    }


def plotly_facts(figure: object) -> dict:
    resolved, unresolved = plotly_figure(figure)
    layout = resolved.get("layout", {})
    traces = [trace for trace in resolved.get("data", []) if trace.get("visible", True) is True]
    panels, marks, styles, xs = set(), [], [], []
    for at, trace in enumerate(traces):
        kind = trace.get("type", "scatter")
        name = trace.get("name") or f"trace {at}"
        mark = "lines" if kind == "scatter" and "lines" in trace.get("mode", "") else kind
        marks.append({"trace": name, "mark": mark, "values": plotly_count(trace)})
        styles.append(plotly_style(name, trace))
        if kind in DOMAIN_TRACES:
            panels.add(("domain", json.dumps(trace.get("domain"), sort_keys=True)))
        elif kind != "table":
            panels.add((trace.get("xaxis", "x"), trace.get("yaxis", "y")))
            xs.append({"trace": name, "x": sequence(trace.get("x"))})
    return {
        "library": "plotly",
        "unresolved": unresolved,
        "canvas": {
            "width": layout.get("width"),
            "height": layout.get("height"),
            "panels": len(panels) or 1,
        },
        "dpi": CSS_DPI,
        "marks": marks,
        "styles": styles,
        "legend": bool(layout.get("showlegend", len(traces) > 1)),
        "xs": xs,
        "texts": plotly_texts(layout),
    }


def matplotlib_figure(figure: object) -> object:
    """The matplotlib Figure behind a Figure, an Axes or a seaborn grid."""
    for attribute in ("figure", "fig"):
        inner = getattr(figure, attribute, None)
        if inner is not None and hasattr(inner, "savefig") and inner is not figure:
            return inner
    return figure


def hex_color(color: tuple) -> str:
    red, green, blue = (round(part * 255) for part in color[:3])
    return f"#{red:02x}{green:02x}{blue:02x}"


def number(value: object) -> object:
    return value.item() if hasattr(value, "item") else value


def line_marks(ax: object, *, dated: bool, found: dict) -> None:
    dates = importlib.import_module("matplotlib.dates")
    for line in ax.lines:
        x = [number(value) for value in line.get_xdata()]
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
            }
        )
        found["xs"].append({"trace": name, "x": x})


def bar_marks(ax: object, found: dict) -> None:
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
            }
        )
        centers = [number(patch.get_x() + patch.get_width() / 2) for patch in bars]
        found["xs"].append({"trace": name, "x": ticks or centers})


def other_marks(ax: object, found: dict) -> None:
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


def matplotlib_facts(figure: object) -> dict:
    dates = importlib.import_module("matplotlib.dates")
    date_formats = (dates.AutoDateFormatter, dates.ConciseDateFormatter, dates.DateFormatter)
    figure = matplotlib_figure(figure)
    figure.canvas.draw()
    width, height = (round(side) for side in figure.get_size_inches() * figure.dpi)
    axes = [ax for ax in figure.axes if ax.get_visible() and ax.get_label() != "<colorbar>"]
    found = {"marks": [], "styles": [], "xs": []}
    for ax in axes:
        dated = isinstance(ax.xaxis.get_major_formatter(), date_formats)
        line_marks(ax, dated=dated, found=found)
        bar_marks(ax, found)
        other_marks(ax, found)
    texts = matplotlib_texts(figure, axes)
    for style in found["styles"]:
        style["direct_labels"] = bool(texts["annotations"])
    return {
        "library": "matplotlib",
        "unresolved": None,
        "canvas": {"width": width, "height": height, "panels": len(axes) or 1},
        "dpi": figure.dpi,
        **found,
        "legend": any(ax.get_legend() is not None for ax in axes) or bool(figure.legends),
        "texts": texts,
    }


def canvas_fact(raw: dict) -> dict:
    canvas = dict(raw["canvas"])
    width, height = canvas["width"], canvas["height"]
    findings = []
    if width and height:
        canvas["shape"] = "taller than wide" if height > width else "wider than tall"
        if canvas["panels"] == 1 and height > width:
            findings.append(f"one panel {width} by {height} pixels is taller than wide")
    return {**canvas, "findings": findings}


def values_fact(raw: dict) -> dict:
    total = sum(mark["values"] for mark in raw["marks"])
    fact = {"marks": raw["marks"], "total": total, "findings": []}
    width, height = raw["canvas"]["width"], raw["canvas"]["height"]
    if width and height:
        fact["per_square_inch"] = round(total / (width * height / raw["dpi"] ** 2), 2)
    if 0 < total <= SMALL_DATA:
        fact["findings"].append(
            f"{total} values drawn, {SMALL_DATA} or fewer, which a table or a sentence can show"
        )
    slices = sum(mark["values"] for mark in raw["marks"] if mark["mark"] in {"pie", "sunburst"})
    if slices:
        fact["findings"].append(f"a pie of {slices} slices")
    return fact


def colors_fact(raw: dict) -> dict:
    styles = raw["styles"]
    labelled = any(style["direct_labels"] for style in styles) or bool(raw["texts"]["annotations"])
    findings = []
    for at, first in enumerate(styles):
        for second in styles[at + 1 :]:
            same = first["dash"] == second["dash"] and first["markers"] == second["markers"]
            pair = {hue(first["color"]), hue(second["color"])} == {"red", "green"}
            if pair and same and not labelled:
                findings.append(
                    f"{first['trace']} and {second['trace']} are told apart only by red against "
                    "green, with the same line style and no direct labels"
                )
    return {
        "traces": styles,
        "legend": raw["legend"],
        "direct_labels": labelled,
        "findings": findings,
    }


def x_fact(raw: dict) -> dict:
    traces, findings = [], []
    for trace in raw["xs"]:
        x = trace["x"]
        shown = [value if isinstance(value, (int, float)) else str(value) for value in x[:50]]
        entry = {"trace": trace["trace"], "count": len(x), "x": shown}
        years = sorted({year for year in map(year_of, x) if year is not None})
        if years and len(years) * 2 >= len({str(value) for value in x}):
            entry["years"] = years
        if len(x) == PAIR:
            around = f", {years[0]} and {years[-1]}," if len(years) == PAIR else ""
            findings.append(
                f"{trace['trace']} has only two x values{around} with nothing before or after them"
            )
        elif len(years) > PAIR:
            span = years[-1] - years[0] + 1
            findings.append(f"{trace['trace']} spans {span} years, {years[0]} to {years[-1]}")
        traces.append(entry)
    return {"traces": traces, "findings": findings}


def text_fact(raw: dict, x_values: dict) -> dict:
    texts = raw["texts"]
    findings = []
    if texts["title"] and capitals(texts["title"]):
        findings.append(f"the title {texts['title']!r} is written in capitals")
    rotated = [angle for angle in texts["tick_angles"] if angle % 180]
    if rotated:
        findings.append(f"tick labels are rotated by {rotated[0]} degrees")
    words = " ".join([texts["title"], *texts["axis_titles"], *texts["annotations"]])
    money = MONEY.search(words)
    if money and not BASIS.search(words):
        traces = x_values["traces"]
        spans = [trace["years"] for trace in traces if len(trace.get("years", [])) > PAIR]
        over = f" over {spans[0][-1] - spans[0][0] + 1} years" if spans else ""
        findings.append(
            f"money ({money.group(0)}){over} with no word that says nominal, real or constant"
        )
    return {**texts, "findings": findings}


def mark_text(mark: dict) -> str:
    return f"{mark['trace']} {mark['values']} {mark['mark']}"


def style_text(style: dict) -> str:
    dash = f" {style['dash']}" if style["dash"] else ""
    markers = " with markers" if style["markers"] else ""
    return f"{style['trace']} {style['color'] or 'mixed colors'}{dash}{markers}"


def trace_text(trace: dict) -> str:
    years = trace.get("years")
    span = f", years {years[0]} to {years[-1]}" if years else ""
    return f"{trace['trace']} {trace['count']} x values{span}"


def summaries(facts: dict) -> dict:
    """One line for each fact, which `xpd chart-review` prints under the questions it answers."""
    canvas = facts["canvas"]
    size = f"{canvas['width']} by {canvas['height']} pixels" if canvas["width"] else "no set size"
    marks = ", ".join(map(mark_text, facts["values"]["marks"]))
    colors = facts["colors"]
    styles = ", ".join(map(style_text, colors["traces"]))
    labels = "direct labels" if colors["direct_labels"] else "no direct labels"
    legend = "a legend" if colors["legend"] else "no legend"
    xs = "; ".join(map(trace_text, facts["x-values"]["traces"]))
    text = facts["text"]
    titles = ", ".join(repr(title) for title in text["axis_titles"]) or "none"
    return {
        "canvas": f"{size}, {canvas['panels']} panel{'s' if canvas['panels'] != 1 else ''}",
        "values": f"{facts['values']['total']} values drawn: {marks or 'none'}",
        "colors": f"{styles or 'no traces'}; {labels}, {legend}",
        "x-values": xs or "no x axis",
        "text": f"title {text['title']!r}, axis titles {titles}",
    }


def facts_of(name: str, raw: dict) -> dict:
    """The five facts of a figure, each with a summary and the findings that name a fault."""
    x_values = x_fact(raw)
    facts = {
        "canvas": canvas_fact(raw),
        "values": values_fact(raw),
        "colors": colors_fact(raw),
        "x-values": x_values,
        "text": text_fact(raw, x_values),
    }
    for fact, summary in summaries(facts).items():
        facts[fact]["summary"] = summary
    return {
        "name": name,
        "library": raw["library"],
        "unresolved": raw["unresolved"],
        "facts": facts,
    }


def measure(name: str, figure: object) -> dict:
    with redirect_stdout(sys.stderr):
        if hasattr(figure, "full_figure_for_development"):
            raw = plotly_facts(figure)
        elif hasattr(matplotlib_figure(figure), "savefig"):
            raw = matplotlib_facts(figure)
        else:
            kind = type(figure).__name__
            fail(f"{name} is a {kind}, which is no plotly or matplotlib figure")
    return facts_of(name, raw)


def adapter_figures(adapter: Path) -> dict:
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
    os.environ.setdefault("MPLBACKEND", "Agg")
    sys.path.insert(0, str(adapter.resolve().parent))
    sys.path.insert(0, str(Path.cwd()))
    measured = []
    for name, figure in adapter_figures(adapter).items():
        try:
            measured.append(measure(str(name), figure))
        except Exception as error:  # noqa: BLE001
            fail(f"cannot measure {name}: {type(error).__name__}: {error}")
    print(json.dumps({"figures": measured}, default=str))


if __name__ == "__main__":
    arguments = sys.argv[1:]
    if len(arguments) != 1:
        fail("usage: chart_facts.py <adapter.py>")
    main(Path(arguments[0]))

"""Color scales without an order, smoothed cells, titles in capitals and charts of a few numbers.

A line that xpd reports ends with a comment that names the rule. Each case returns the
figures the code builds.
"""

import base64
import struct
import zlib

import plotly.express as px
import plotly.graph_objects as go
import plotly.io as pio
from plotly.subplots import make_subplots

pio.templates.default = "simple_white"

CASES = {}
GRID = [[0, 10], [10, 0]]
SCALE = "Jet"


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def checker_png():
    raw = b"\x00\x00\x00\x00\xff\xff\xff\x00\xff\xff\xff\x00\x00\x00"

    def chunk(kind, body):
        return struct.pack(">I", len(body)) + kind + body + struct.pack(">I", zlib.crc32(kind + body) & 0xFFFFFFFF)

    data = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b"")
    return "data:image/png;base64," + base64.b64encode(data).decode()


@case
def spectrum_scales():
    heat = go.Figure(go.Heatmap(z=GRID, colorscale="Jet"))  # xpd: rainbow-colorscale
    image = px.imshow(GRID, color_continuous_scale="turbo")  # xpd: rainbow-colorscale
    dots = px.scatter(x=[1, 2, 3], y=[1, 2, 3], color=[1, 2, 3], color_continuous_scale=px.colors.sequential.Rainbow)  # xpd: rainbow-colorscale
    marker = go.Figure(go.Scatter(x=[1, 2], y=[1, 2], mode="markers", marker=dict(color=[1, 2], colorscale="hsv_r")))  # xpd: rainbow-colorscale
    later = px.imshow(GRID)
    later.update_layout(coloraxis_colorscale="Jet")  # xpd: rainbow-colorscale
    in_dict = px.imshow(GRID).update_layout(coloraxis={"colorscale": "Rainbow_r"})  # xpd: rainbow-colorscale
    one_way = go.Figure(go.Heatmap(z=GRID, colorscale="Viridis"))
    gray = px.imshow(GRID, color_continuous_scale="Greys")
    by_name = go.Figure(go.Heatmap(z=GRID, colorscale=SCALE))
    discrete = px.pie(names=["a", "b"], values=[1, 2], color_discrete_sequence=px.colors.sequential.Jet)  # xpd: pie-chart, table-for-small-data
    sampled = px.colors.sample_colorscale("Jet", [0, 0.5, 1])
    settings = dict(colorscale="Jet")
    return heat, image, dots, marker, later, in_dict, one_way, gray, by_name, discrete, sampled, settings


@case
def default_scale():
    before = px.defaults.color_continuous_scale
    px.defaults.color_continuous_scale = px.colors.sequential.Turbo  # xpd: rainbow-colorscale
    fig = px.imshow(GRID)
    px.defaults.color_continuous_scale = before
    return fig


@case
def smoothing():
    best = go.Figure(go.Heatmap(z=GRID, zsmooth="best"))  # xpd: heatmap-smoothing
    fast = go.Figure(go.Heatmap(z=GRID, zsmooth="fast"))  # xpd: heatmap-smoothing
    counts = go.Figure(go.Histogram2d(x=[0, 1, 0, 1], y=[0, 0, 1, 1], zsmooth="best"))  # xpd: heatmap-smoothing
    counts_fast = go.Figure(go.Histogram2d(x=[0, 1, 0, 1], y=[0, 0, 1, 1], zsmooth="fast"))
    picture = go.Figure(go.Image(source=checker_png(), zsmooth="fast"))  # xpd: heatmap-smoothing
    pixels = go.Figure(go.Image(z=[[[0, 0, 0], [255, 255, 255]]], zsmooth="fast"))
    later = px.density_heatmap(x=[0, 1, 0, 1], y=[0, 0, 1, 1])
    later.update_traces(zsmooth="best")  # xpd: heatmap-smoothing
    later_fast = px.density_heatmap(x=[0, 1, 0, 1], y=[0, 0, 1, 1])
    later_fast.update_traces(zsmooth="fast")
    blocks = go.Figure(go.Heatmap(z=GRID, zsmooth=False))
    return best, fast, counts, counts_fast, picture, pixels, later, later_fast, blocks


@case
def capital_titles():
    shares = dict(region=["north", "south"], share=[41, 59])
    titled = px.bar(shares, x="region", y="share", title="SALES BY REGION")  # xpd: all-caps-title
    later = go.Figure(go.Bar(x=shares["region"], y=shares["share"]))
    later.update_layout(
        title_text="MONTHLY ACTIVE USERS",  # xpd: all-caps-title
        xaxis_title="MONTH OF YEAR",  # xpd: all-caps-title
        yaxis_title="GDP (USD)",
    )
    bold = px.bar(shares, x="region", y="share").update_layout(title=dict(text="<b>TOTAL SALES</b>"))  # xpd: all-caps-title
    labelled = px.bar(shares, x="region", y="share", labels={"share": "SHARE OF VOTES"})  # xpd: all-caps-title
    panels = make_subplots(rows=1, cols=2, subplot_titles=("NORTH REGION", "South region"))  # xpd: all-caps-title
    axis = px.bar(shares, x="region", y="share")
    axis.update_xaxes(title_text="SALES REGION")  # xpd: all-caps-title
    short = px.bar(shares, x="region", y="share", title="RSI")
    one_word = px.bar(shares, x="region", y="share", title="NVIDIA")
    mixed = px.bar(shares, x="region", y="share", title="Sales by region, USD")
    column = px.bar(shares, x="region", y="share", labels={"SHARE OF VOTES": "Share of votes"})
    note = px.bar(shares, x="region", y="share").add_annotation(text="CONFIDENTIAL DRAFT", x=0, y=0)
    year = 2024
    field = px.bar(shares, x="region", y="share", title=f"SALES IN {year}")
    return titled, later, bold, labelled, panels, axis, short, one_word, mixed, column, note, field


@case
def few_numbers():
    names = ["north", "south", "east", "west"]
    values = [41.3, 37.8, 12.6, 8.3]
    bars = px.bar(x=names, y=[41.3, 37.8, 12.6, 8.3])  # xpd: table-for-small-data
    lying = go.Figure(go.Bar(x=(3, -5), y=["a", "b"], orientation="h"))  # xpd: table-for-small-data
    pie = go.Figure(go.Pie(labels=names, values=[413, 378, 126, 83]))  # xpd: pie-chart, table-for-small-data
    px_pie = px.pie(names=["a", "b", "c"], values=(1, 2, 3))  # xpd: pie-chart, table-for-small-data
    by_name = go.Figure(go.Bar(x=names, y=values))
    many = go.Figure(go.Bar(y=[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21]))
    gaps = go.Figure(go.Bar(x=["a", "b", "c"], y=[1, None, 3]))
    names_only = px.bar(x=["a", "b"], y=["c", "d"])
    in_list = go.Figure(data=[go.Pie(labels=["a", "b"], values=[3, 1])])  # xpd: pie-chart, table-for-small-data
    ratings = go.Figure(go.Bar(x=[1, 2, 3, 4, 5], y=values + [4.0]))
    upright_positions = go.Figure(go.Bar(x=[1, 2], y=["a", "b"]))
    two_traces = go.Figure(data=[go.Bar(x=["a", "b"], y=[1, 2]), go.Bar(x=["a", "b"], y=[3, 4])])
    added = go.Figure()
    added.add_trace(go.Bar(x=["a", "b"], y=[1, 2]))
    written = go.Figure(go.Bar(x=["a", "b"], y=[1, 2], text=[1, 2]))
    return bars, lying, pie, px_pie, by_name, many, gaps, names_only, in_list, ratings, upright_positions, two_traces, added, written

"""Charts drawn from a frame with matplotlib and seaborn, and the kind passed as an argument.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import matplotlib

matplotlib.use("Agg")  # xpd: import-side-effect
import matplotlib.pyplot as plt
import pandas as pd
import seaborn as sns

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def frame():
    return pd.DataFrame({"team": ["a", "b", "c"], "score": [3, 5, 4], "cost": [1.0, 2.5, 2.0]})


@case
def pyplot_on_two_columns():
    df = frame()
    points = plt.scatter(df["score"], df["cost"])  # xpd: matplotlib-on-frame
    line = plt.plot(df["score"], df["cost"], color="black")  # xpd: matplotlib-on-frame
    bars = plt.bar(df["team"], df["score"])  # xpd: matplotlib-on-frame
    return points.get_offsets().tolist(), line[0].get_ydata().tolist(), [bar.get_height() for bar in bars]


@case
def pyplot_on_one_column():
    df = frame()
    counts, _, _ = plt.hist(df["score"], bins=4)  # xpd: matplotlib-on-frame
    return counts.tolist()


@case
def seaborn_on_a_frame():
    df = frame()
    plt.figure()
    points = sns.scatterplot(data=df, x="score", y="cost", hue="team")  # xpd: matplotlib-on-frame
    plt.figure()
    bars = sns.histplot(data=df, x="score")  # xpd: matplotlib-on-frame
    plt.figure()
    box = sns.boxplot(data=df, y="score")  # xpd: matplotlib-on-frame
    return points.collections[0].get_offsets().tolist(), len(bars.patches), len(box.lines)


@case
def kind_as_an_argument():
    df = frame()
    bar = df.plot(kind="bar", x="team", y="score", title="Score", backend="plotly")  # xpd: plot-kind-argument
    line = df.plot(x="team", y="score", kind="line", backend="plotly")  # xpd: plot-kind-argument
    area = df["score"].plot(kind="area", backend="plotly")  # xpd: plot-kind-argument
    return bar, line, area


@case
def left_alone():
    """Values that are no columns of one frame, a mean per group, and a kind without a method."""
    df = frame()
    plt.figure()
    points = plt.scatter([1, 2, 3], df["cost"])
    labels_first = plt.barh(df["team"], df["score"])
    means = sns.barplot(data=df, x="team", y="score")
    violin = df.plot(kind="violin", y="score", backend="plotly")
    kind = "bar"
    by_name = df.plot(kind=kind, x="team", y="score", backend="plotly")
    return points.get_offsets().tolist(), len(labels_first), len(means.patches), violin, by_name

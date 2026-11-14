"""Python work per value on arrays.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import math

import numpy as np

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def bigger(a, b):
    return a if a > b else b


@case
def vectorize():
    a, b = np.arange(10.0), np.full(10, 4.5)
    return np.vectorize(bigger)(a, b)  # xpd: np-vectorize


@np.vectorize  # xpd: np-vectorize
def clipped(x):
    return min(x, 3.0)


@case
def vectorize_decorator():
    return clipped(np.arange(6.0))


@case
def element_loops():
    a, b = np.arange(8.0), np.linspace(0, 1, 8)
    out = np.empty_like(a)
    for i in range(len(a)):  # xpd: element-loop
        out[i] = a[i] * 2 + np.sqrt(b[i])
    grid = np.zeros((3, 4))
    for i in range(3):
        for j in range(4):  # xpd: element-loop
            grid[i, j] = i * 10 + j
    count = np.zeros(8)
    k = 0
    while k < len(a):  # xpd: element-loop
        count[k] = a[k] - b[k]
        k += 1
    doubled = a.copy()
    for x in np.nditer(doubled, op_flags=["readwrite"]):  # xpd: element-loop
        x[...] = x * 2
    shifted = np.empty((2, 3))
    for idx, v in np.ndenumerate(grid[:2, :3]):  # xpd: element-loop
        shifted[idx] = v + 1
    return out, grid, count, doubled, shifted


@case
def meant_loops():
    a = np.arange(8.0)
    acc = np.zeros(8)
    for i in range(1, 8):
        acc[i] = acc[i - 1] + a[i]
    labels = np.empty(8, dtype=object)
    for i in range(8):
        labels[i] = str(a[i])
    rows = np.zeros((3, 4))
    for i in range(3):
        rows[i, : i + 1] = 1
    names = [0] * 8
    for i in range(8):
        names[i] = a[i] * 2
    chunks = a.copy()
    for x in np.nditer(chunks, flags=["external_loop"], op_flags=["readwrite"]):
        x[...] = x * 2
    k = 0
    while k < 8:
        acc[k] += 1
        k += 2
    return acc, labels, rows, names, chunks


@case
def growth():
    pieces = [np.arange(3.0) + i for i in range(4)]
    out = np.array([])
    for p in pieces:
        out = np.append(out, p)  # xpd: growth-in-loop
    stacked = pieces[0]
    for p in pieces[1:]:
        stacked = np.vstack((stacked, p))  # xpd: growth-in-loop
    return out, stacked


@case
def meant_growth():
    pieces = [np.arange(3.0) + i for i in range(4)]
    collected = []
    for p in pieces:
        collected.append(p)
    whole = np.concatenate(collected)
    seen = np.array([0.0])
    for p in pieces:
        if np.isin(p[0], seen).any():
            continue
        seen = np.concatenate((seen, p[:1]))
    padded = []
    for p in pieces:
        row = p * 2
        row = np.concatenate((row, [0.0]))
        padded.append(row)
    return whole, seen, padded


@case
def comprehensions():
    a = np.linspace(1, 4, 7)
    squares = np.array([x * x + 1 for x in a])  # xpd: per-value-comprehension
    roots = np.array([math.sqrt(x) for x in a])  # xpd: per-value-comprehension
    steps = np.array([i * 0.5 for i in range(6)])  # xpd: per-value-comprehension
    mapped = list(map(np.sqrt, a))  # xpd: per-value-comprehension
    return squares, roots, steps, mapped


def scale(x):
    return x * 3


@case
def meant_comprehensions():
    a = np.linspace(1, 4, 7)
    custom = np.array([scale(x) for x in a])
    kept = np.array([x for x in a if x > 2])
    powers = np.array([a**i for i in range(3)])
    copies = np.array([x for x in a])
    lengths = list(map(np.asarray, (a, a)))
    return custom, kept, powers, copies, lengths


@case
def builtins():
    a = np.linspace(-2, 3, 11)
    total = sum(np.abs(a))  # xpd: builtin-on-array
    over = sum(np.abs(a) > 1)  # xpd: builtin-on-array
    peak = max(np.abs(a))  # xpd: builtin-on-array
    low = min(np.sqrt(np.abs(a)) < 1)  # xpd: builtin-on-array
    ordered = sorted(np.abs(a))  # xpd: builtin-on-array
    values = list(np.arange(4))  # xpd: builtin-on-array
    return total, over, peak, low, ordered, values


@case
def meant_builtins():
    a = np.linspace(-2, 3, 11)
    whole = sum([1, 2, 3])
    shape_max = max(a.shape)
    hits = any(np.abs(a) > 1)
    magnitude = abs(a - 1)
    found = sum(np.where(a > 0))
    return whole, shape_max, hits, magnitude, found

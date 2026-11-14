"""NumPy code that runs and computes the wrong thing, or whose mistake is certain from the text.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file. A form
that raises on NumPy 2.5 sits in a function that is not a case.
"""

import math

import numpy as np

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def data():
    return np.array([1.0, np.nan, 3.0])


@case
def nan_comparisons():
    x = data()
    return [
        x == np.nan,  # xpd: compare-to-nan
        np.nan == x,  # xpd: compare-to-nan
        x != np.nan,  # xpd: compare-to-nan
        x == float("nan"),  # xpd: compare-to-nan
        float("nan") != x,  # xpd: compare-to-nan
        x != math.nan,  # xpd: compare-to-nan
        np.nan in np.unique(x),  # xpd: compare-to-nan
        np.isnan(x),
        ~np.isnan(x),
        x == np.inf,
        np.nan in [np.nan],
        np.isnan(np.unique(x)).any(),
    ]


def nan_in_a_test():
    x = data()
    np.testing.assert_array_equal(x == np.nan, np.zeros(3, bool))


def wrong_slots_that_raise(a, b, n):
    return [
        np.zeros(3, 4),  # xpd: argument-in-wrong-slot
        np.ones(2, 5),  # xpd: argument-in-wrong-slot
        np.full(3, 4, 0),  # xpd: argument-in-wrong-slot
        np.reshape(a, 2, 3),  # xpd: argument-in-wrong-slot
        np.vstack(a, b),  # xpd: argument-in-wrong-slot
        np.hstack(a, b, a),  # xpd: argument-in-wrong-slot
        np.concatenate(a, [4.0, 5.0]),  # xpd: argument-in-wrong-slot
        np.random.rand((3, 4)),  # xpd: argument-in-wrong-slot
        np.random.random(3, 4),  # xpd: argument-in-wrong-slot
        np.linspace(0, 1, 0.1),  # xpd: argument-in-wrong-slot
        np.linspace(0, 1, 1.0 / n),  # xpd: argument-in-wrong-slot
        np.quantile(a, 95),  # xpd: argument-in-wrong-slot
        np.nanquantile(a, [25, 75], axis=0),  # xpd: argument-in-wrong-slot
        np.quantile(a, q=95, axis=0),  # xpd: argument-in-wrong-slot
        np.linspace(0, 1, num=0.5),  # xpd: argument-in-wrong-slot
    ]


@case
def wrong_slots_look_alikes():
    a = np.arange(6.0)
    return [
        np.zeros((3, 4)),
        np.zeros(3, int),
        np.zeros(3, dtype=np.float32),
        np.full((3, 4), 0),
        np.full(3, 4, float),
        np.reshape(a, (2, 3)),
        np.reshape(a, (2, 3), "F"),
        np.vstack((a, a)),
        np.vstack([a, a], dtype=float),
        np.concatenate((a, a)),
        np.concatenate([a.reshape(2, 3), a.reshape(2, 3)], 1),
        np.stack([a, a], -1),
        np.random.default_rng(0).random((3, 4)).shape,
        np.linspace(0, 1, 11),
        np.linspace(0, 1, 6 // 2),
        np.linspace(0, 1, num=5),
        np.quantile(a, 0.95),
        np.quantile(a, 1),
        np.quantile(a, [0.25, 0.75]),
        np.percentile(a, 95),
    ]


@case
def percentile_of_a_fraction():
    x = np.arange(1.0, 101.0)
    return [
        np.percentile(x, 0.95),  # xpd: percentile-of-fraction
        np.nanpercentile(x, [0.25, 0.75]),  # xpd: percentile-of-fraction
        np.percentile(x, 95),
        np.percentile(x, [0.5, 99.5]),
        np.percentile(x, 1.5),
        np.quantile(x, 0.95),
    ]


@case
def tolerance_in_the_third_slot():
    return [
        np.isclose(0.0, 1e-4, 1e-3),  # xpd: positional-tolerance
        np.allclose(np.zeros(2), np.full(2, 1e-4), 1e-3),  # xpd: positional-tolerance
        np.isclose(0.0, 1e-4, atol=1e-3),
        np.isclose(0.0, 1e-4, rtol=1e-3),
        np.allclose(np.zeros(2), np.full(2, 1e-4)),
        np.allclose(np.zeros(2), np.full(2, 1e-4), 0, 1e-3),
        np.isclose(0.0, 1e-4, 1e-3, atol=0),
    ]


def tolerances_named_by_the_caller(a, b, rtol, tols):
    return np.isclose(a, b, rtol), np.allclose(a, b, **tols)


@case
def float_steps():
    return [
        np.arange(1, 1.3, 0.1),  # xpd: arange-float-step
        np.arange(0, 1, step=0.05),  # xpd: arange-float-step
        np.arange(0.0, 1.0, 1e-1),  # xpd: arange-float-step
        np.arange(0, 1, 0.25),
        np.arange(0, 2, 0.5),
        np.arange(0.5, 10),
        np.arange(10),
        np.linspace(1, 1.2, 3),
    ]


@case
def one_value_only():
    rng = np.random.default_rng(0)
    np.random.seed(0)
    return [
        np.random.randint(0, 1, 5),  # xpd: randint-single-value
        np.random.randint(1),  # xpd: randint-single-value
        rng.integers(0, 1, size=5),  # xpd: randint-single-value
        np.random.randint(0, 2, 5),
        rng.integers(0, 1, size=5, endpoint=True),
        rng.integers(0, 10),
    ]


@case
def results_that_are_none():
    x = np.array([3, 1, 2])
    y = np.array([3, 1, 2])
    m = np.ones((2, 2))
    rng = np.random.default_rng(0)
    x = x.sort()  # xpd: result-is-none
    s = y.sort()  # xpd: result-is-none
    m = np.fill_diagonal(m, 0)  # xpd: result-is-none
    z = np.zeros(3, int)
    copied = np.copyto(z, np.array([1, 2, 3]))  # xpd: result-is-none
    y = rng.shuffle(y)  # xpd: result-is-none
    w = np.array([5, 6])
    w = w.fill(0)  # xpd: result-is-none
    return [x, s, m, copied, y, w]


def sorted_and_returned(v):
    return v.sort()  # xpd: result-is-none


@case
def results_kept():
    x = np.array([3, 1, 2])
    x.sort()
    y = np.sort(np.array([3, 1, 2]))
    m = np.ones((2, 2))
    np.fill_diagonal(m, 0)
    rng = np.random.default_rng(0)
    p = rng.permutation(np.arange(3))
    w = np.array([5, 6])
    w.fill(0)
    v, i = np.array([2, 1]), 0
    return [x, y, m, p, w, v, i]


def sorted_by_a_namespace(xp, values, cursor):
    ordered = xp.sort(values)
    by_axis = xp.sort(values, axis=-1)
    rows = cursor.sort("name")
    return ordered, by_axis, rows


@case
def results_thrown_away():
    x = np.array([3, 1, 2])
    m = np.ones((2, 3))
    np.sort(x)  # xpd: result-discarded
    np.append(x, 4)  # xpd: result-discarded
    np.clip(x, 0, 2)  # xpd: result-discarded
    x.astype(float)  # xpd: result-discarded
    m.reshape(3, 2)  # xpd: result-discarded
    m.round(1)  # xpd: result-discarded
    return [x, m]


@case
def results_written_in_place():
    x = np.array([3.0, -1.0, 2.0])
    m = np.ones((2, 3))
    np.abs(x, x)
    np.clip(x, 0, 2, out=x)
    np.round(x, 1, x)
    np.nan_to_num(x, copy=False)
    np.random.shuffle(x)
    x = np.sort(x)
    m = m.reshape(3, 2)
    x.sort()
    x.clip(0, 1, x)
    try:
        np.asarray(m)
    except TypeError:
        pass
    m.transpose()
    return [x, m]


def time_sort(x):
    np.sort(x)


@case
def writes_that_land_in_a_copy():
    x = np.arange(6.0)
    m = np.zeros((2, 2))
    x[x > 2][0] = -1  # xpd: write-to-copy
    x[[0, 1]][0] = -1  # xpd: write-to-copy
    x[(x > 1) & (x < 4)][:] = 0  # xpd: write-to-copy
    x[np.isfinite(x)][0] += 1  # xpd: write-to-copy
    m.flatten()[0] = 1  # xpd: write-to-copy
    x.astype(int)[0] = 9  # xpd: write-to-copy
    x[np.isfinite(x) & (x > 2)][0] = 1  # xpd: write-to-copy
    return [x, m]


@case
def writes_that_reach_the_array():
    x = np.arange(6.0)
    m = np.arange(6.0).reshape(2, 3)
    grid = [[0, 0], [0, 0]]
    i = 0
    grid[i ^ 1][1] += 1
    x[1:4][0] = 9
    m[0][m[0] > 1] = 0
    m[1][2] = -5
    grid[0][1] = 7
    x[np.flatnonzero(x > 2)[0]] = -1
    m.ravel()[0] = 4
    return [x, m, grid]


def masks_combined_with_and(x):
    a = x[(x > 2) and (x < 8)]  # xpd: array-truth-value
    b = x[(x < 2) or (x > 8)]  # xpd: array-truth-value
    c = x[not (x > 2)]  # xpd: array-truth-value
    d = x[2 < x < 8]  # xpd: array-truth-value
    return a, b, c, d


def mask_precedence(x):
    return x[x > 2 & x < 8]  # xpd: mask-operator-precedence


@case
def masks_combined_right():
    x = np.array([1, 5, 9])
    flags = 6
    return [
        x[(x > 2) & (x < 8)],
        x[(x < 2) | (x > 8)],
        x[~(x > 2)],
        x[(2 < x) & (x < 8)],
        flags & 4 == 4,
        flags == 2 | 4,
        x[x > 2] if x.size and x[0] > 0 else x,
    ]


@case
def int_masks():
    y = np.array([10, 20, 30])
    return [
        y[(y > 15).astype(int)],  # xpd: int-mask-as-index
        y[np.isnan(y).astype(np.int64)],  # xpd: int-mask-as-index
        ~(y > 15).astype(int),  # xpd: int-mask-as-index
        y[y > 15],
        ~(y > 15),
        1 - (y > 15).astype(int),
        y[np.array([0, 2]).astype(int)],
        np.array(["low", "high"])[(y > 15).astype(int)],
    ]


def kind(value):
    return type(value).__name__, np.shape(value), getattr(value, "dtype", None)


@case
def iterators_in_numpy():
    y = [1, 2, 3]
    d = {"a": 1, "b": 2}
    return [
        kind(np.array(v * 2 for v in y)),  # xpd: iterator-to-array
        kind(np.array(map(float, y))),  # xpd: iterator-to-array
        kind(np.asarray(zip(y, y), dtype=object)),  # xpd: iterator-to-array
        kind(np.array(d.values())),  # xpd: iterator-to-array
        kind(np.array({1, 2, 3})),  # xpd: iterator-to-array
        kind(np.max(v for v in y)),  # xpd: iterator-to-array
        np.argmax(map(abs, y)),  # xpd: iterator-to-array
        np.any(v > 100 for v in y),  # xpd: iterator-to-array
        np.array([v * 2 for v in y]),
        np.array(list(map(float, y))),
        np.fromiter((v * 2 for v in y), float),
        np.array(range(3)),
        np.array(sorted({1, 2, 3})),
        np.max([v for v in y]),
        np.array(list(d.values())),
        max(v for v in y),
        np.array(np.arange(3)),
    ]


def iterators_that_raise(y):
    np.sum(v for v in y)  # xpd: iterator-to-array
    np.mean(map(float, y))  # xpd: iterator-to-array
    np.concatenate(np.ones(2) for _ in y)  # xpd: iterator-to-array


@case
def nan_cast_to_int():
    x = np.array([1.5, 2.5, 3.5])
    return [
        np.where(x > 2, x, np.nan).astype(int),  # xpd: nan-to-int
        np.full(2, np.nan).astype(np.int64),  # xpd: nan-to-int
        np.where(x > 2, x, 0).astype(int),
        np.where(x > 2, x, np.nan).astype(float),
        np.nan_to_num(np.where(x > 2, x, np.nan)).astype(int),
    ]


def reads_unset_memory(n):
    total = np.empty(n).sum()  # xpd: read-uninitialized
    shifted = np.empty_like(np.arange(n)) + 1  # xpd: read-uninitialized
    return total, shifted


@case
def reads_set_memory():
    out = np.empty(3)
    out[:] = 1
    return [
        np.empty(3) * np.nan,
        np.zeros(3).sum(),
        out,
        np.empty(0).sum(),
    ]


def add_one(acc=np.zeros(2)):  # xpd: array-default-argument
    acc += 1
    return acc


def set_first(values=np.ones(3)):  # xpd: array-default-argument
    values[0] = 0
    return values


def scaled(values=np.ones(3)):
    return values * 2


def reset(acc=None):
    acc = np.zeros(2) if acc is None else acc
    acc += 1
    return acc


@case
def shared_defaults():
    add_one()
    set_first()
    return [add_one(), set_first(), scaled(), reset()]


def row_inserted(m):
    return np.insert(m, 0, [[5, 5]])  # xpd: append-flattens


@case
def rows_appended():
    m = np.ones((2, 2))
    return [
        np.append(m, [[5, 5]]),  # xpd: append-flattens
        np.append(m, [[5, 5]], axis=0),
        np.append(m, [[5], [5]], 1),
        np.append(np.arange(3), [4, 5]),
    ]


@case
def compared_reductions():
    a, b = np.array([1, 2]), np.array([5, 9])
    return [
        a.all() == b.all(),  # xpd: compare-reductions
        np.any(a) == np.any(b),  # xpd: compare-reductions
        np.array_equal(a, b),
        a.all() and b.all(),
        a.sum() == b.sum(),
    ]


@case
def where_as_a_count():
    z = np.array([0, 5, 0, 7])
    found = "yes" if np.where(z > 9) else "no"  # xpd: where-tuple-length
    if np.nonzero(z > 9):  # xpd: where-tuple-length
        found += "!"
    return [
        len(np.where(z > 0)),  # xpd: where-tuple-length
        len(np.nonzero(z)),  # xpd: where-tuple-length
        len(np.where(z > 0)[0]),
        np.count_nonzero(z > 0),
        len(np.where(z > 0, z, 0)),
        found,
        np.where(z > 9)[0].size > 0,
    ]

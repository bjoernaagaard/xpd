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

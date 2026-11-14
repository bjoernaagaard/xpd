"""Whole-array math that picks a slow or numerically weak function.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


rng = np.random.default_rng(0)
x = rng.standard_normal(200)
y = rng.standard_normal(200)
grid = rng.standard_normal((20, 10))
n = 5


@case
def sort_then_select():
    return (
        np.sort(x)[0],  # xpd: np-sort-then-select
        np.sort(x)[-1],  # xpd: np-sort-then-select
        np.sort(x)[:5],  # xpd: np-sort-then-select
        np.sort(x)[-5:],  # xpd: np-sort-then-select
        np.sort(x)[::-1][:5],  # xpd: np-sort-then-select
        x[np.argsort(x)],  # xpd: np-sort-then-select
        np.argsort(x)[0],  # xpd: np-sort-then-select
        np.argsort(x)[-1],  # xpd: np-sort-then-select
        np.argsort(x)[:5],  # xpd: np-sort-then-select
        np.argsort(x)[-5:],  # xpd: np-sort-then-select
        np.argsort(x)[::-1][:5],  # xpd: np-sort-then-select
        np.argsort(x)[-5:][::-1],  # xpd: np-sort-then-select
        np.argsort(np.argsort(x)),  # xpd: np-sort-then-select
        np.sort(x)[1],
        np.sort(grid, axis=0)[0],
        np.sort(x)[:n],
        np.sort(x)[:150],
        np.sort(x)[::-1],
        y[np.argsort(x)],
        np.argsort(x)[1],
        np.argsort(x)[:n],
        np.argsort(x, kind="stable")[:5],
        np.partition(x, 5)[:5],
    )


@case
def count_nonzero():
    mask = x > 0
    return (
        np.sum(x > 0.5),  # xpd: np-count-nonzero
        len(np.where(x > 0)[0]),  # xpd: np-count-nonzero
        len(np.nonzero(mask)[0]),  # xpd: np-count-nonzero
        len(np.flatnonzero(mask)),  # xpd: np-count-nonzero
        len(x[x > 0]),  # xpd: np-count-nonzero
        np.sum(x > 2) > 0,  # xpd: np-count-nonzero
        np.sum(y < -2) > 0,  # xpd: np-count-nonzero
        np.sum(mask),
        np.sum(x * y),  # xpd: np-sum-of-products
        np.sum(x > 0, axis=0),
        np.sum(x > 0) > 1,  # xpd: np-count-nonzero
        len(np.where(x > 0)),
        len(x[mask]),
        np.count_nonzero(x > 0),
        np.any(x > 2),
    )


@case
def where_shortcut():
    return (
        np.where(x > 0, True, False),  # xpd: np-where-shortcut
        np.where(x > 0, 1, 0),  # xpd: np-where-shortcut
        np.where(x > 0, x, 0),  # xpd: np-where-shortcut
        np.where(x >= 0, x, 0.0),  # xpd: np-where-shortcut
        np.where(x < 0, 0, x),  # xpd: np-where-shortcut
        np.arange(len(x))[x > 1],  # xpd: np-where-shortcut
        np.where(x > 0, False, True),
        np.where(x > 0, 2, 0),
        np.where(x > 0, y, 0),
        np.where(x > 1, x, 1),
        np.where(x < 0, x, 0),
        np.arange(len(x))[5:],
        np.flatnonzero(x > 1),
    )


small = np.abs(x) * 1e-6
prob = np.abs(x) / 10


@case
def expm1_log1p():
    return (
        np.exp(small) - 1,  # xpd: np-expm1-log1p
        1 - np.exp(-small),  # xpd: np-expm1-log1p
        np.log(1 + small),  # xpd: np-expm1-log1p
        np.log(small + 1.0),  # xpd: np-expm1-log1p
        np.log(1 - prob),  # xpd: np-expm1-log1p
        np.exp(small) - 2,
        np.exp(5) - 1,
        np.log(1 + 1e-3),
        np.exp(small - 1),
        np.log(2 + small),
        np.log(1 - prob * 2),
        np.expm1(small),
        np.log1p(-prob),
    )


@case
def logaddexp():
    s = np.exp(x) / np.sum(np.exp(x))  # xpd: np-logaddexp
    t = np.exp(grid) / np.sum(np.exp(grid), axis=1, keepdims=True)  # xpd: np-logaddexp
    return (
        np.log(np.exp(x) + np.exp(y)),  # xpd: np-logaddexp
        np.log(1 + np.exp(x)),  # xpd: np-logaddexp
        np.log1p(np.exp(x)),  # xpd: np-logaddexp
        np.log(np.sum(np.exp(x))),  # xpd: np-logaddexp
        s,
        t,
        np.log(np.exp(x) + y),
        np.log(np.sum(np.exp(x - np.max(x)))),
        np.exp(x - np.max(x)) / np.sum(np.exp(x - np.max(x))),
        np.logaddexp(0, x),
    )


@case
def log_of_product():
    return (
        np.log(np.prod(np.abs(x) + 1)),  # xpd: np-log-of-product
        np.log(np.prod(grid, axis=0)),
        np.log(np.sum(np.abs(x))),
        np.sum(np.log(np.abs(x) + 1)),  # xpd: np-expm1-log1p
        np.sum(np.log(np.abs(x) + 2)),
    )



@case
def reduce_last():
    return (
        np.cumsum(x)[-1],  # xpd: np-reduce-last
        np.cumprod(prob)[-1],  # xpd: np-reduce-last
        np.cumsum(x)[-2],
        np.cumsum(x, axis=0)[-1],
        np.cumsum(x),
    )



@case
def power_half():
    return (
        np.power(np.abs(x), 0.5),  # xpd: np-power-half
        np.power(np.abs(x), 2),
        np.abs(x) ** 0.5,
    )


@case
def sum_of_products():
    return (
        np.sum(x * y),  # xpd: np-sum-of-products
        np.sum(grid * grid),  # xpd: np-sum-of-products
        np.sum(x**2),  # xpd: np-sum-of-products
        np.sum(np.square(y)),  # xpd: np-sum-of-products
        np.mean((x - y) ** 2),  # xpd: np-sum-of-products
        np.sum(2 * x),
        np.sum((x > 0) * (y > 0)),
        np.sum(x[:, None] * grid[0]),
        np.sum(x * x.conj()),
        np.sum(x * 2.5),
        np.sum(x * y, axis=0),
        np.sum(x**3),
        np.mean((x - y) ** 3),
        np.vdot(x, y),
    )


@case
def norm():
    return (
        np.sqrt(np.sum(x**2)),  # xpd: np-norm
        np.sqrt(np.sum(x * x)),  # xpd: np-norm
        np.sqrt(np.sum(np.square(grid))),  # xpd: np-norm
        np.sqrt((x**2).sum()),  # xpd: np-norm
        np.sqrt(np.sum(x**2) / x.size),  # xpd: np-sum-of-products
        np.sqrt(np.sum(x * y)),  # xpd: np-sum-of-products
        np.sqrt(np.sum(x**2, axis=0)),
        np.linalg.norm(x),
    )


@case
def vecdot():
    return (
        np.sum(grid * grid[::-1], axis=-1),
        np.sum(grid**2, axis=-1),  # xpd: np-vecdot
        np.sum(np.square(grid), axis=-1),  # xpd: np-vecdot
        np.sum(grid * grid, axis=1),
        np.sum(grid * 2, axis=-1),
        np.sum(grid**2, axis=0),
        np.vecdot(grid, grid),
    )

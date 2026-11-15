"""SciPy calls made once per value or once per pair, where one call on whole arrays exists.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from scipy import fft, fftpack, linalg, spatial, special, stats  # xpd: legacy-fftpack
from scipy.spatial import KDTree, cKDTree, distance
from scipy.spatial.distance import cdist, cityblock, pdist
from scipy.special import gammaln
from scipy.stats import norm

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def points():
    rng = np.random.default_rng(0)
    return rng.random((5, 3)), rng.random((4, 3))


@case
def distances_per_pair():
    X, Y = points()
    a = np.array([[distance.euclidean(x, y) for y in Y] for x in X])  # xpd: distance-per-pair
    b = np.empty((len(X), len(Y)))
    for i in range(len(X)):
        for j in range(len(Y)):
            b[i, j] = cityblock(X[i], Y[j])  # xpd: distance-per-pair
    c = min(distance.cosine(X[0], y) for y in Y)  # xpd: distance-per-pair
    return a, b, c


@case
def distances_in_one_call():
    X, Y = points()
    a = cdist(X, Y)
    b = distance.cdist(X, Y, "cityblock")
    c = distance.cosine(X[0], Y[0])
    d = [np.linalg.norm(x - y) for x in X for y in Y]
    return a, b, c, d


@case
def metric_as_function():
    X, Y = points()
    a = cdist(X, Y, distance.euclidean)  # xpd: callable-metric
    b = distance.pdist(X, metric=cityblock)  # xpd: callable-metric
    c = distance.cdist(X, Y, metric=distance.sqeuclidean)  # xpd: callable-metric
    return a, b, c


@case
def metric_as_name():
    X, Y = points()
    a = cdist(X, Y, "euclidean")
    b = pdist(X, metric=lambda u, v: float(np.abs(u - v).max()))
    c = distance.cdist(X, Y, metric="sqeuclidean")
    return a, b, c


@case
def special_per_value():
    x = np.linspace(0.5, 3.0, 6)
    a = np.array([special.gamma(v) for v in x])  # xpd: per-value-scipy-call
    b = sum(gammaln(v) for v in x)  # xpd: per-value-scipy-call
    c = [special.erf(v) for v in x]  # xpd: per-value-scipy-call
    return a, b, c


@case
def special_on_arrays():
    x = np.linspace(0.5, 3.0, 6)
    rows = [x, x + 1.0]
    a = special.gamma(x)
    b = [special.logsumexp(r) for r in rows]
    c = [special.gamma(v) for v in x if v > 1.0]
    d = [special.gamma(v) + 1.0 for v in x]
    return a, b, c, d


@case
def stats_per_value():
    x = np.linspace(-2.0, 2.0, 5)
    a = np.array([stats.norm.cdf(v) for v in x])  # xpd: per-value-scipy-call
    b = [norm.sf(v, loc=1.0) for v in x]  # xpd: per-value-scipy-call
    c = np.array([stats.t.ppf(q, 5) for q in (0.9, 0.95, 0.99)])  # xpd: per-value-scipy-call
    return a, b, c


@case
def stats_on_arrays():
    x = np.linspace(-2.0, 2.0, 5)
    samples = [x, x * 2.0]
    a = stats.norm.cdf(x)
    b = [stats.norm.fit(s) for s in samples]
    c = [stats.ttest_1samp(s, 0.0).pvalue for s in samples]
    return a, b, c


@case
def query_per_point():
    X, Y = points()
    tree = KDTree(X)
    a = np.array([tree.query(q)[1] for q in Y])  # xpd: query-per-point
    other = spatial.cKDTree(X)  # xpd: ckdtree
    b = np.empty(len(Y))
    for i, q in enumerate(Y):
        b[i] = other.query(q, k=1)[0]  # xpd: query-per-point
    c = [tree.query(Y[j], k=2)[1] for j in range(len(Y))]  # xpd: query-per-point
    return a, b, c


@case
def query_all_points():
    X, Y = points()
    tree = KDTree(X)
    a = tree.query(Y)[1]
    trees = [KDTree(X), KDTree(Y)]
    b = [t.query(X[0]) for t in trees]
    c = cKDTree  # the class itself, as code that checks isinstance does
    return a, b, c.__name__


@case
def solve_per_pass():
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    sides = np.array([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
    xs = np.array([linalg.solve(a, b) for b in sides])  # xpd: solve-in-loop
    lu = linalg.lu_factor(a)
    ys = np.array([linalg.lu_solve(lu, b) for b in sides])
    zs = [linalg.solve(m, sides[0]) for m in (a, 2 * a)]
    ws = np.linalg.solve(a, sides.T).T
    return xs, ys, zs, ws


@case
def transforms():
    x = np.linspace(0.0, 1.0, 8)
    return fftpack.dct(x), fft.dct(x), fft.fft(x)


@case
def frozen_per_value():
    x = np.linspace(-2.0, 2.0, 5)
    means = np.array([0.0, 0.5, 1.0])
    a = np.array([stats.norm(m, 2.0).pdf(x) for m in means])  # xpd: per-value-scipy-call
    b = [norm(loc=m).cdf(0.0) for m in means]  # xpd: per-value-scipy-call
    c = stats.norm.pdf(x, loc=means[:, None], scale=2.0)
    frozen = stats.norm(0.0, 2.0)
    d = [frozen.pdf(v) for v in x]
    return a, b, c, d



@case
def percentiles_per_value():
    x = np.linspace(0.0, 10.0, 11)
    a = [stats.scoreatpercentile(x, p) for p in (10, 50, 90)]  # xpd: per-value-scipy-call
    b = stats.scoreatpercentile(x, [10, 50, 90])
    c = [stats.scoreatpercentile(row, 50) for row in (x, 2 * x)]
    return a, b, c

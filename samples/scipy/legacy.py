"""SciPy calls where the name still exists but an argument, a default or the entry point is old.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file. A form
that raises or warns on SciPy 1.18.1 sits in a function that is not a case.
"""

import io

import numpy as np
import scipy
import scipy.sparse as sparse
import scipy.sparse.linalg as spla
from scipy import integrate, interpolate, linalg, ndimage, optimize, signal, stats
from scipy.cluster.vq import kmeans2
from scipy.integrate import odeint
from scipy.io import hb_read, mmread, mmwrite
from scipy.optimize import minimize
from scipy.spatial import cKDTree
from scipy.spatial.distance import cdist, directed_hausdorff
from scipy.spatial.transform import Rotation
from scipy.sparse import random as sparse_random
from scipy.sparse.linalg import gmres
from scipy.stats import qmc

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def removed_keywords(a, b, y, x, points):
    spla.cg(a, b, tol=1e-8)  # xpd: removed-keyword
    gmres(a, b, atol="legacy")  # xpd: removed-keyword
    scipy.sparse.linalg.bicgstab(a, b, None, 1e-8)  # xpd: removed-keyword
    gmres(a, b, restrt=5)  # xpd: removed-keyword
    linalg.solve(a, b, sym_pos=True)  # xpd: removed-keyword
    linalg.solve(a, b, sym_pos=False)  # xpd: removed-keyword
    linalg.eigh(a, eigvals=(0, 1))  # xpd: removed-keyword
    linalg.eigh(a, turbo=True)  # xpd: removed-keyword
    linalg.pinv(a, rcond=1e-10)  # xpd: removed-keyword
    integrate.simpson(y, x=x, even="avg")  # xpd: removed-keyword
    integrate.cumulative_trapezoid(y, x, initial=1.0)  # xpd: removed-keyword
    tree = cKDTree(points)  # xpd: ckdtree
    tree.query(points[:2], n_jobs=2)  # xpd: removed-keyword
    cKDTree(points).query_ball_point(points[0], 0.5, n_jobs=-1)  # xpd: ckdtree, removed-keyword
    stats.kendalltau(x, y, initial_lexsort=True)  # xpd: removed-keyword
    stats.iqr(y, scale="raw")  # xpd: removed-keyword
    scipy.optimize.fmin_l_bfgs_b(np.sum, x, approx_grad=True, iprint=-1)  # xpd: removed-keyword, legacy-optimize
    cdist(points, points, "wminkowski", p=2, w=np.ones(2))  # xpd: removed-keyword
    opts = {}
    spla.cg(a, b, tol=1e-8, **opts)  # xpd: removed-keyword
    gmres(a, b, restrt=5, restart=5)  # xpd: removed-keyword
    linalg.solve(a, b, sym_pos=True, assume_a="gen")  # xpd: removed-keyword
    linalg.eigh(a, eigvals=(0, 1), **opts)  # xpd: removed-keyword
    tree.query_pairs(0.5, n_jobs=2)  # xpd: removed-keyword


@case
def current_keywords():
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    b = np.array([1.0, 2.0])
    x = np.linspace(0.0, 1.0, 11)
    y = np.sin(3 * x)
    points = np.c_[x, y]
    tree = cKDTree(points)  # xpd: ckdtree
    return (
        spla.cg(a, b, rtol=1e-8)[0],
        gmres(a, b, atol=0.0, restart=5)[0],
        scipy.sparse.linalg.bicgstab(a, b, None)[0],
        linalg.solve(a, b, assume_a="pos"),
        linalg.eigh(a, subset_by_index=(0, 1))[0],
        linalg.pinv(a, rtol=1e-10),
        integrate.simpson(y, x=x),
        integrate.cumulative_trapezoid(y, x, initial=0),
        integrate.cumulative_trapezoid(y, x, initial=0.0),
        tree.query(points[:2], workers=2)[0],
        stats.kendalltau(x, y).statistic,
        stats.iqr(y, scale=1.0),
        cdist(points, points, "minkowski", p=2, w=np.ones(2)),
    )


def deprecated_arguments(t, y, freqs, a, samples):
    signal.lombscargle(t, y, freqs, precenter=True)  # xpd: deprecated-argument
    signal.lombscargle(t, y, freqs, precenter=False)  # xpd: deprecated-argument
    signal.lombscargle(t, y, freqs, False, True)  # xpd: deprecated-argument
    linalg.qr(a, lwork=None)  # xpd: deprecated-argument
    linalg.qr(a, False, -1)  # xpd: deprecated-argument
    stats.anderson_ksamp(samples, midrank=False)  # xpd: deprecated-argument
    stats.anderson_ksamp(samples, True)  # xpd: deprecated-argument
    optimize.linprog([1, 1], A_ub=[[-1, -1]], b_ub=[-1], method="interior-point")  # xpd: deprecated-argument


@case
def current_arguments():
    t = np.linspace(0.0, 1.0, 20)
    y = np.sin(t)
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    rng = np.random.default_rng(0)
    samples = [rng.standard_normal(50), rng.standard_normal(50) + 0.5]
    return (
        signal.lombscargle(t, y - y.mean(), np.array([1.0, 2.0]), normalize=True),
        linalg.qr(a, mode="economic")[1],
        stats.anderson_ksamp(samples, variant="midrank").statistic,
        optimize.linprog([1, 1], A_ub=[[-1, -1]], b_ub=[-1], method="highs").fun,
    )


def defaults_that_warn(a, b, x, image, path):
    gmres(a, b, callback=print)  # xpd: default-that-warns
    stats.anderson(x)  # xpd: default-that-warns
    stats.anderson_ksamp([x, x + 1])  # xpd: default-that-warns
    ndimage.zoom(image, 2, grid_mode=True)  # xpd: default-that-warns
    ndimage.zoom(image, 2, mode="wrap", grid_mode=True)  # xpd: default-that-warns
    return path


@case
def defaults_kept():
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    b = np.array([1.0, 2.0])
    seen = []
    x = gmres(a, b, callback=seen.append)[0]  # xpd: default-that-warns
    buffer = io.BytesIO()
    mmwrite(buffer, sparse.coo_matrix(np.eye(3)))  # xpd: sparse-matrix-api
    buffer.seek(0)
    m = mmread(buffer)  # xpd: default-that-warns
    return x, len(seen), m.toarray(), type(m).__name__


@case
def defaults_chosen():
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    b = np.array([1.0, 2.0])
    x = np.random.default_rng(0).standard_normal(50)
    buffer = io.BytesIO()
    mmwrite(buffer, sparse.coo_matrix(np.eye(3)))  # xpd: sparse-matrix-api
    buffer.seek(0)
    return (
        gmres(a, b, callback=len, callback_type="x")[0],
        stats.anderson(x, method="interpolate").pvalue,
        mmread(buffer, spmatrix=False).toarray(),
        ndimage.zoom(np.ones((4, 4)), 2, grid_mode=True, mode="grid-constant"),
        ndimage.zoom(np.ones((4, 4)), 2, mode="nearest", grid_mode=True),
    )


@case
def defaults_passed_through_a_mapping():
    a = np.array([[4.0, 1.0], [1.0, 3.0]])
    b = np.array([1.0, 2.0])
    opts = {"callback_type": "legacy"}
    seen = []
    x = gmres(a, b, callback=seen.append, **opts)[0]
    buffer = io.BytesIO()
    mmwrite(buffer, sparse.coo_array(np.eye(3)))
    buffer.seek(0)
    m = mmread(buffer, **{"spmatrix": True})
    return x, len(seen), m.toarray(), type(m).__name__


@case
def sum_on_a_rebound_name():
    import scipy.ndimage as ndi

    def total(ndi, image):
        return ndi.sum(image)

    return total(ndi, np.arange(4.0))


def read_harwell_boeing(path):
    return hb_read(path)  # xpd: default-that-warns


def mode_indexing(x):
    value = stats.mode(x)[0][0]
    count = stats.mode(x)[1][0]
    first = stats.mode(x).mode[0]
    return value, count, first


@case
def mode_kept():
    x = np.array([1, 2, 2, 3])
    rows = np.array([[1, 5], [1, 6], [2, 6]])
    return (
        stats.mode(x, keepdims=True)[0][0],
        stats.mode(x)[0],
        stats.mode(rows, axis=0)[0][0],
        stats.mode(x).mode,
    )


def minimize_options(f, g, x0):
    minimize(f, x0, method="L-BFGS-B", options={"disp": True, "maxiter": 50})  # xpd: minimize-unknown-option
    optimize.minimize(f, x0, method="TNC", options=dict(maxiter=50))  # xpd: minimize-unknown-option
    minimize(f, x0, method="BFGS", options={"maxfun": 50})  # xpd: minimize-unknown-option
    minimize(f, x0, method="dogleg", options={"eta": 0.1})
    minimize(f, x0, method="trust-constr", options={"ftol": 1e-9})  # xpd: minimize-unknown-option
    minimize(f, x0, method="Nelder-Mead", jac=g)  # xpd: minimize-unused-argument
    minimize(f, x0, method="BFGS", jac=g, hess=g)  # xpd: minimize-unused-argument
    minimize(f, x0, method="CG", bounds=[(0, 1), (0, 1)])  # xpd: minimize-unused-argument
    minimize(f, x0, method="L-BFGS-B", constraints=[{"type": "ineq", "fun": g}])  # xpd: minimize-unused-argument


@case
def minimize_known_options():
    def f(v):
        return float(np.sum((np.asarray(v) - 1.0) ** 2))

    def g(v):
        return 2 * (np.asarray(v) - 1.0)

    x0 = [0.0, 0.0]
    return (
        minimize(f, x0, method="L-BFGS-B", options={"maxiter": 50}).x,
        minimize(f, x0, method="TNC", jac=g, options={"maxfun": 50}).x,
        minimize(f, x0, method="Nelder-Mead", jac=None, options={"maxfev": 400}).x,
        minimize(f, x0, method="trust-ncg", jac=g, hess=lambda v: 2 * np.eye(2), options={"gtol": 1e-8}).x,
        minimize(f, x0, method="L-BFGS-B", bounds=[(0, 2), (0, 2)]).x,
        minimize(f, x0, bounds=[(0, 2), (0, 2)]).x,
    )


def rng_keywords(x, f, bounds, sp_version=(1, 14)):
    sparse.random(5, 5, density=0.5, random_state=0)  # xpd: rng-keyword, sparse-matrix-api
    sparse_random(5, 5, 0.5, random_state=0)  # xpd: rng-keyword
    kmeans2(x, 3, seed=1)  # xpd: rng-keyword
    optimize.differential_evolution(f, bounds, seed=2)  # xpd: rng-keyword
    qmc.Sobol(2, seed=3)  # xpd: rng-keyword
    stats.bootstrap((x,), np.mean, random_state=4)  # xpd: rng-keyword
    spla.svds(x, k=1, random_state=5)  # xpd: rng-keyword
    interpolate.BarycentricInterpolator(x, x, random_state=6)  # xpd: rng-keyword
    linalg.clarkson_woodruff_transform(x, 2, seed=7)  # xpd: rng-keyword
    directed_hausdorff(x, x, seed=8)  # xpd: rng-keyword
    Rotation.random(2, random_state=9)  # xpd: rng-keyword
    if sp_version < (1, 15):
        stats.bootstrap((x,), np.mean, random_state=4)


@case
def rng_by_keyword():
    x = np.random.default_rng(0).standard_normal((40, 2))
    return (
        sparse.random(5, 5, density=0.5, rng=0).toarray(),  # xpd: sparse-matrix-api
        kmeans2(x, 3, rng=1)[1],
        qmc.Sobol(2, rng=3).random(4),
        stats.norm.rvs(size=3, random_state=np.random.default_rng(4)),
    )


def legacy_entry_points(f, x0, x, y, t, y0, sos, image, labels):
    optimize.fmin(f, x0)  # xpd: legacy-optimize
    optimize.leastsq(f, x0)  # xpd: legacy-optimize
    optimize.brent(f)  # xpd: legacy-optimize
    odeint(f, y0, t)  # xpd: legacy-odeint
    interpolate.interp1d(x, y)  # xpd: legacy-interpolate
    interpolate.splev(x, interpolate.splrep(x, y))  # xpd: legacy-interpolate, legacy-interpolate
    interpolate.UnivariateSpline(x, y)  # xpd: legacy-interpolate
    interpolate.Rbf(x, y)  # xpd: legacy-interpolate
    signal.stft(y, nperseg=16)  # xpd: legacy-signal
    stats.ranksums(x, y)  # xpd: legacy-api
    sparse.bmat([[None]])  # xpd: legacy-api, sparse-matrix-api


@case
def legacy_aliases():
    sos = signal.butter(2, 0.3, output="sos")
    image = np.arange(12.0).reshape(3, 4)
    labels = np.array([[1, 1, 2, 2], [1, 2, 2, 0], [0, 0, 1, 1]])
    return (
        signal.sosfreqz(sos, worN=16),  # xpd: legacy-signal
        ndimage.sum(image, labels, [1, 2]),  # xpd: legacy-api
        scipy.ndimage.sum(image),  # xpd: legacy-api
        signal.freqz_sos(sos, worN=16),
        ndimage.sum_labels(image, labels, [1, 2]),
    )


@case
def current_entry_points():
    x = np.linspace(0.0, 1.0, 11)
    y = np.sin(3 * x)
    return (
        optimize.minimize(lambda v: float((v[0] - 1) ** 2), [0.0], method="Nelder-Mead").x,
        optimize.minimize_scalar(lambda v: (v - 1) ** 2, method="brent").x,
        optimize.least_squares(lambda v: v - 1.0, [0.0]).x,
        integrate.solve_ivp(lambda t, v: -v, (0.0, 1.0), [1.0]).y[:, -1],
        np.interp(0.5, x, y),
        interpolate.make_interp_spline(x, y, k=3)(0.5),
        interpolate.RBFInterpolator(x[:, None], y)(np.array([[0.5]])),
        stats.mannwhitneyu(x, y, use_continuity=False, method="asymptotic").pvalue,
        sparse.block_array([[sparse.eye_array(2), None], [None, sparse.eye_array(2)]]).toarray(),
    )


def look_alikes(tol, model):
    model.fit(tol=1e-8)
    model.query([0.0], n_jobs=2)
    np.linalg.pinv(np.eye(2), rcond=1e-10)
    model.minimize(method="BFGS", options={"maxfun": 5})
    model.mode(tol)[0][0]
    stats.mode(tol, 0, "propagate", False)[0][0]
    model.sum(tol)
    np.random.default_rng(0).random(3)
    np.interp(0.5, [0.0, 1.0], [0.0, 1.0])
    return tol

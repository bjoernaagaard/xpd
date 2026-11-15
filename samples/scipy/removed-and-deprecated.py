"""SciPy names that are gone or that warn.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file. The
private-module blocks at the end are written by
research/scipy-rules/removed/gen_sample.py from the module table.
"""

import numpy as np
import scipy
import scipy as sp
import scipy.integrate as si
import scipy.misc  # xpd: removed-in-scipy
import scipy.odr  # xpd: deprecated-in-scipy
import scipy.signal
import scipy.spatial
import scipy.stats
from scipy import integrate, interpolate, linalg, signal, sparse, spatial, special, stats
from scipy import misc  # xpd: removed-in-scipy
from scipy import odr as old_odr  # xpd: deprecated-in-scipy
from scipy.integrate import trapezoid
from scipy.spatial import Delaunay
import scipy.constants.codata  # xpd: deprecated-in-scipy
from scipy.constants.codata import physical_constants as physical_constants_constants_codata  # xpd: deprecated-in-scipy
from scipy.constants import physical_constants as physical_constants_constants_codata_now
import scipy.fftpack.basic  # xpd: deprecated-in-scipy, legacy-fftpack
from scipy.fftpack.basic import fft as fft_fftpack_basic  # xpd: deprecated-in-scipy, legacy-fftpack
from scipy.fftpack import fft as fft_fftpack_basic_now  # xpd: legacy-fftpack
import scipy.integrate.odepack  # xpd: deprecated-in-scipy
from scipy.integrate.odepack import odeint as odeint_integrate_odepack  # xpd: deprecated-in-scipy
from scipy.integrate import odeint as odeint_integrate_odepack_now
import scipy.interpolate.fitpack  # xpd: deprecated-in-scipy
from scipy.interpolate.fitpack import BSpline as BSpline_interpolate_fitpack  # xpd: deprecated-in-scipy
from scipy.interpolate import BSpline as BSpline_interpolate_fitpack_now
import scipy.io.arff.arffread  # xpd: deprecated-in-scipy
from scipy.io.arff.arffread import MetaData as MetaData_io_arff_arffread  # xpd: deprecated-in-scipy
from scipy.io.arff import MetaData as MetaData_io_arff_arffread_now
import scipy.io.matlab.mio  # xpd: deprecated-in-scipy
from scipy.io.matlab.mio import loadmat as loadmat_io_matlab_mio  # xpd: deprecated-in-scipy
from scipy.io.matlab import loadmat as loadmat_io_matlab_mio_now
import scipy.io.harwell_boeing  # xpd: deprecated-in-scipy
from scipy.io.harwell_boeing import hb_read as hb_read_io_harwell_boeing  # xpd: deprecated-in-scipy
from scipy.io import hb_read as hb_read_io_harwell_boeing_now
import scipy.linalg.basic  # xpd: deprecated-in-scipy
from scipy.linalg.basic import solve as solve_linalg_basic  # xpd: deprecated-in-scipy
from scipy.linalg import solve as solve_linalg_basic_now
import scipy.ndimage.filters  # xpd: deprecated-in-scipy
from scipy.ndimage.filters import correlate1d as correlate1d_ndimage_filters  # xpd: deprecated-in-scipy
from scipy.ndimage import correlate1d as correlate1d_ndimage_filters_now
import scipy.optimize.cobyla  # xpd: deprecated-in-scipy
from scipy.optimize.cobyla import OptimizeResult as OptimizeResult_optimize_cobyla  # xpd: deprecated-in-scipy
from scipy.optimize import OptimizeResult as OptimizeResult_optimize_cobyla_now
import scipy.signal.windows.windows  # xpd: deprecated-in-scipy
from scipy.signal.windows.windows import boxcar as boxcar_signal_windows_windows  # xpd: deprecated-in-scipy
from scipy.signal.windows import boxcar as boxcar_signal_windows_windows_now
import scipy.signal.bsplines  # xpd: deprecated-in-scipy
from scipy.signal.bsplines import spline_filter as spline_filter_signal_bsplines  # xpd: deprecated-in-scipy
from scipy.signal import spline_filter as spline_filter_signal_bsplines_now
import scipy.sparse.linalg.dsolve  # xpd: deprecated-in-scipy
from scipy.sparse.linalg.dsolve import MatrixRankWarning as MatrixRankWarning_sparse_linalg_dsolve  # xpd: deprecated-in-scipy
from scipy.sparse.linalg import MatrixRankWarning as MatrixRankWarning_sparse_linalg_dsolve_now
import scipy.sparse.base  # xpd: deprecated-in-scipy
from scipy.sparse.base import SparseEfficiencyWarning as SparseEfficiencyWarning_sparse_base  # xpd: deprecated-in-scipy
from scipy.sparse import SparseEfficiencyWarning as SparseEfficiencyWarning_sparse_base_now
import scipy.spatial.transform.rotation  # xpd: deprecated-in-scipy
from scipy.spatial.transform.rotation import Rotation as Rotation_spatial_transform_rotation  # xpd: deprecated-in-scipy
from scipy.spatial.transform import Rotation as Rotation_spatial_transform_rotation_now
import scipy.spatial.ckdtree  # xpd: deprecated-in-scipy
from scipy.spatial.ckdtree import cKDTree as cKDTree_spatial_ckdtree  # xpd: deprecated-in-scipy
from scipy.spatial import cKDTree as cKDTree_spatial_ckdtree_now
import scipy.special.basic  # xpd: deprecated-in-scipy
from scipy.special.basic import ai_zeros as ai_zeros_special_basic  # xpd: deprecated-in-scipy
from scipy.special import ai_zeros as ai_zeros_special_basic_now
import scipy.stats.kde  # xpd: deprecated-in-scipy
from scipy.stats.kde import gaussian_kde as gaussian_kde_stats_kde  # xpd: deprecated-in-scipy
from scipy.stats import gaussian_kde as gaussian_kde_stats_kde_now
import scipy.stats.mstats_basic  # xpd: deprecated-in-scipy
from scipy.stats.mstats_basic import argstoarray as argstoarray_stats_mstats_basic  # xpd: deprecated-in-scipy
from scipy.stats.mstats import argstoarray as argstoarray_stats_mstats_basic_now

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def removed_integration_names(y):
    a = si.trapz(y)  # xpd: removed-in-scipy
    b = integrate.cumtrapz(y)  # xpd: removed-in-scipy
    c = scipy.integrate.simps(y)  # xpd: removed-in-scipy
    d = sp.integrate.quadrature(np.sin, 0, 1)  # xpd: removed-in-scipy
    from scipy.integrate import trapz  # xpd: removed-in-scipy
    from scipy.integrate import cumtrapz as ctz  # xpd: removed-in-scipy
    from scipy.integrate import romberg  # xpd: removed-in-scipy
    return a, b, c, d, trapz, ctz, romberg


@case
def current_integration_names():
    y = np.array([1.0, 2.0, 4.0])
    return si.trapezoid(y), integrate.simpson(y), integrate.cumulative_trapezoid(y), trapezoid(y), np.trapezoid(y)


def removed_and_moved_names(x):
    w = signal.hann(8)  # xpd: removed-in-scipy
    k = scipy.signal.kaiser(8, 5.0)  # xpd: removed-in-scipy
    from scipy.signal import tukey  # xpd: removed-in-scipy
    gen = stats.NumericalInverseHermite(stats.norm())  # xpd: removed-in-scipy
    rep = stats.find_repeats(x)  # xpd: removed-in-scipy
    p = linalg.pinv2(x)  # xpd: removed-in-scipy
    from scipy.linalg import pinv2 as pv  # xpd: removed-in-scipy
    e = sparse.speye(3)  # xpd: removed-in-scipy
    m = stats.median_absolute_deviation(x)  # xpd: removed-in-scipy
    return w, k, tukey, gen, rep, p, pv, e, m


@case
def current_moved_names():
    x = np.array([1.0, 2.0, 2.0, 5.0])
    return (signal.windows.hann(8), scipy.signal.windows.kaiser(8, 5.0), signal.get_window("hann", 8),
            stats.mstats.find_repeats(x)[0], linalg.pinv(np.eye(2)), sparse.eye(3).toarray(),  # xpd: sparse-matrix-api
            stats.median_abs_deviation(x))


def removed_without_one_to_one(x):
    a = interpolate.interp2d([0, 1], [0, 1], [[0, 1], [1, 2]])  # xpd: removed-in-scipy
    from scipy.interpolate import interp2d  # xpd: removed-in-scipy
    b = signal.cwt(x, signal.ricker, [1, 2])  # xpd: removed-in-scipy, removed-in-scipy
    c = special.sph_harm(0, 1, 0.5, 0.5)  # xpd: removed-in-scipy
    from scipy.special import lpmn  # xpd: removed-in-scipy
    d = stats.binom_test(3, 10)  # xpd: removed-in-scipy
    e = linalg.triu(x)  # xpd: removed-in-scipy
    f = sparse.cs_graph_components(x)  # xpd: removed-in-scipy
    g = scipy.spatial.distance.wminkowski(x, x, 2, x)  # xpd: removed-in-scipy
    h = scipy.optimize.anneal(np.sin, 0.0)  # xpd: removed-in-scipy
    i = scipy.ndimage.imread("face.png")  # xpd: removed-in-scipy
    from scipy.stats import itemfreq  # xpd: removed-in-scipy
    return a, interp2d, b, c, lpmn, d, e, f, g, h, i, itemfreq


@case
def current_names_without_one_to_one():
    x = np.array([[1.0, 2.0], [3.0, 4.0]])
    return (special.sph_harm_y(1, 0, 0.5, 0.5), stats.binomtest(3, 10).pvalue, np.triu(x),
            interpolate.RegularGridInterpolator(([0, 1], [0, 1]), x)([0.5, 0.5]),
            scipy.spatial.distance.minkowski(x[0], x[1], 2, w=[1.0, 2.0]))


def misc_names():
    a = misc.comb(5, 2)  # xpd: removed-in-scipy
    b = scipy.misc.face()  # xpd: removed-in-scipy
    c = misc.imread("face.png")  # xpd: removed-in-scipy
    from scipy.misc import logsumexp  # xpd: removed-in-scipy
    return a, b, c, logsumexp


@case
def current_misc_homes():
    return special.comb(5, 2), special.logsumexp([1.0, 2.0])


def numpy_names_on_scipy(x):
    a = scipy.pi * x  # xpd: removed-in-scipy
    b = sp.array([1, 2])  # xpd: removed-in-scipy
    c = scipy.sqrt(x)  # xpd: removed-in-scipy
    from scipy import arange  # xpd: removed-in-scipy
    d = scipy.comb(5, 2)  # xpd: removed-in-scipy
    from scipy import factorial  # xpd: removed-in-scipy
    return a, b, c, arange, d, factorial


@case
def current_numpy_names():
    x = np.array([1.0, 4.0])
    return np.pi * x, np.sqrt(x), scipy.__version__ > "0", scipy.linalg.norm(x), scipy.fft.fft(x)


def deprecated_calls(x, tri, pts):
    a = spatial.minkowski_distance(x, x)  # xpd: deprecated-in-scipy
    b = scipy.spatial.distance_matrix(x, x)  # xpd: deprecated-in-scipy
    from scipy.spatial import minkowski_distance_p  # xpd: deprecated-in-scipy
    c = interpolate.lagrange([0, 1, 2], [1, 3, 7])  # xpd: deprecated-in-scipy
    d = interpolate.pade([1, 1, 0.5], 1)  # xpd: deprecated-in-scipy
    e = interpolate.approximate_taylor_polynomial(np.exp, 0, 3, 1)  # xpd: deprecated-in-scipy
    f = scipy.cluster.vq.py_vq(x, x)  # xpd: deprecated-in-scipy
    g = spatial.tsearch(tri, xi=pts)  # xpd: deprecated-in-scipy
    from scipy.spatial import tsearch  # xpd: deprecated-in-scipy
    h = old_odr.Model(np.sin)
    return a, b, minkowski_distance_p, c, d, e, f, g, tsearch, h


@case
def tsearch_on_a_triangulation():
    tri = Delaunay(np.array([[0, 0], [1, 0], [0, 1], [1, 1.0]]))
    pts = np.array([[0.2, 0.2], [0.9, 0.8], [5.0, 5.0]])
    holder = {"t": tri}
    a = spatial.tsearch(tri, pts)  # xpd: deprecated-in-scipy
    b = scipy.spatial.tsearch(holder["t"], pts[:2])  # xpd: deprecated-in-scipy
    c = tri.find_simplex(pts)
    return a, b, c


@case
def current_distance_names():
    x = np.array([[0.0, 0.0], [1.0, 1.0]])
    return scipy.spatial.distance.cdist(x, x), scipy.spatial.distance.minkowski(x[0], x[1])

@case
def private_constants_codata():
    old = scipy.constants.codata.physical_constants  # xpd: deprecated-in-scipy
    now = scipy.constants.physical_constants
    return old, now, physical_constants_constants_codata, physical_constants_constants_codata_now


def private_constants_codata_other_forms():
    from scipy.constants import codata  # xpd: deprecated-in-scipy
    from scipy.constants.codata import k  # xpd: deprecated-in-scipy
    gone = scipy.constants.codata.k  # xpd: deprecated-in-scipy
    return codata, k, gone

@case
def private_fftpack_basic():
    old = scipy.fftpack.basic.fft  # xpd: deprecated-in-scipy
    now = scipy.fftpack.fft
    return old, now, fft_fftpack_basic, fft_fftpack_basic_now


def private_fftpack_basic_other_forms():
    from scipy.fftpack import basic  # xpd: deprecated-in-scipy, legacy-fftpack
    from scipy.fftpack.basic import no_such_name  # xpd: deprecated-in-scipy, legacy-fftpack
    gone = scipy.fftpack.basic.no_such_name  # xpd: deprecated-in-scipy
    return basic, no_such_name, gone

@case
def private_integrate_odepack():
    old = scipy.integrate.odepack.odeint  # xpd: deprecated-in-scipy
    now = scipy.integrate.odeint
    return old, now, odeint_integrate_odepack, odeint_integrate_odepack_now


def private_integrate_odepack_other_forms():
    from scipy.integrate import odepack  # xpd: deprecated-in-scipy
    from scipy.integrate.lsoda import lsoda  # xpd: deprecated-in-scipy
    gone = scipy.integrate.lsoda.lsoda  # xpd: deprecated-in-scipy
    return odepack, lsoda, gone

@case
def private_interpolate_fitpack():
    old = scipy.interpolate.fitpack.BSpline  # xpd: deprecated-in-scipy
    now = scipy.interpolate.BSpline
    return old, now, BSpline_interpolate_fitpack, BSpline_interpolate_fitpack_now


def private_interpolate_fitpack_other_forms():
    from scipy.interpolate import fitpack  # xpd: deprecated-in-scipy
    from scipy.interpolate.dfitpack import spalde  # xpd: deprecated-in-scipy
    gone = scipy.interpolate.dfitpack.spalde  # xpd: deprecated-in-scipy
    return fitpack, spalde, gone

@case
def private_io_arff_arffread():
    old = scipy.io.arff.arffread.MetaData  # xpd: deprecated-in-scipy
    now = scipy.io.arff.MetaData
    return old, now, MetaData_io_arff_arffread, MetaData_io_arff_arffread_now


def private_io_arff_arffread_other_forms():
    from scipy.io.arff import arffread  # xpd: deprecated-in-scipy
    from scipy.io.arff.arffread import no_such_name  # xpd: deprecated-in-scipy
    gone = scipy.io.arff.arffread.no_such_name  # xpd: deprecated-in-scipy
    return arffread, no_such_name, gone

@case
def private_io_matlab_mio():
    old = scipy.io.matlab.mio.loadmat  # xpd: deprecated-in-scipy
    now = scipy.io.matlab.loadmat
    return old, now, loadmat_io_matlab_mio, loadmat_io_matlab_mio_now


def private_io_matlab_mio_other_forms():
    from scipy.io.matlab import mio  # xpd: deprecated-in-scipy
    from scipy.io.matlab.mio import no_such_name  # xpd: deprecated-in-scipy
    gone = scipy.io.matlab.mio.no_such_name  # xpd: deprecated-in-scipy
    return mio, no_such_name, gone

@case
def private_io_harwell_boeing():
    old = scipy.io.harwell_boeing.hb_read  # xpd: deprecated-in-scipy
    now = scipy.io.hb_read
    return old, now, hb_read_io_harwell_boeing, hb_read_io_harwell_boeing_now


def private_io_harwell_boeing_other_forms():
    from scipy.io import harwell_boeing  # xpd: deprecated-in-scipy
    from scipy.io.mmio import mminfo  # xpd: deprecated-in-scipy
    gone = scipy.io.mmio.mminfo  # xpd: deprecated-in-scipy
    return harwell_boeing, mminfo, gone

@case
def private_linalg_basic():
    old = scipy.linalg.basic.solve  # xpd: deprecated-in-scipy
    now = scipy.linalg.solve
    return old, now, solve_linalg_basic, solve_linalg_basic_now


def private_linalg_basic_other_forms():
    from scipy.linalg import basic  # xpd: deprecated-in-scipy
    from scipy.linalg.basic import py  # xpd: deprecated-in-scipy
    gone = scipy.linalg.basic.py  # xpd: deprecated-in-scipy
    return basic, py, gone

@case
def private_ndimage_filters():
    old = scipy.ndimage.filters.correlate1d  # xpd: deprecated-in-scipy
    now = scipy.ndimage.correlate1d
    return old, now, correlate1d_ndimage_filters, correlate1d_ndimage_filters_now


def private_ndimage_filters_other_forms():
    from scipy.ndimage import filters  # xpd: deprecated-in-scipy
    from scipy.ndimage.filters import generate_binary_structure  # xpd: deprecated-in-scipy
    gone = scipy.ndimage.filters.generate_binary_structure  # xpd: deprecated-in-scipy
    return filters, generate_binary_structure, gone

@case
def private_optimize_cobyla():
    old = scipy.optimize.cobyla.OptimizeResult  # xpd: deprecated-in-scipy
    now = scipy.optimize.OptimizeResult
    return old, now, OptimizeResult_optimize_cobyla, OptimizeResult_optimize_cobyla_now


def private_optimize_cobyla_other_forms():
    from scipy.optimize import cobyla  # xpd: deprecated-in-scipy
    from scipy.optimize.lbfgsb import zeros  # xpd: deprecated-in-scipy
    gone = scipy.optimize.lbfgsb.zeros  # xpd: deprecated-in-scipy
    return cobyla, zeros, gone

@case
def private_signal_windows_windows():
    old = scipy.signal.windows.windows.boxcar  # xpd: deprecated-in-scipy
    now = scipy.signal.windows.boxcar
    return old, now, boxcar_signal_windows_windows, boxcar_signal_windows_windows_now


def private_signal_windows_windows_other_forms():
    from scipy.signal.windows import windows  # xpd: deprecated-in-scipy
    from scipy.signal.windows.windows import no_such_name  # xpd: deprecated-in-scipy
    gone = scipy.signal.windows.windows.no_such_name  # xpd: deprecated-in-scipy
    return windows, no_such_name, gone

@case
def private_signal_bsplines():
    old = scipy.signal.bsplines.spline_filter  # xpd: deprecated-in-scipy
    now = scipy.signal.spline_filter
    return old, now, spline_filter_signal_bsplines, spline_filter_signal_bsplines_now


def private_signal_bsplines_other_forms():
    from scipy.signal import bsplines  # xpd: deprecated-in-scipy
    from scipy.signal.signaltools import _centered  # xpd: deprecated-in-scipy
    gone = scipy.signal.signaltools._centered  # xpd: deprecated-in-scipy
    return bsplines, _centered, gone

@case
def private_sparse_linalg_dsolve():
    old = scipy.sparse.linalg.dsolve.MatrixRankWarning  # xpd: deprecated-in-scipy
    now = scipy.sparse.linalg.MatrixRankWarning
    return old, now, MatrixRankWarning_sparse_linalg_dsolve, MatrixRankWarning_sparse_linalg_dsolve_now


def private_sparse_linalg_dsolve_other_forms():
    from scipy.sparse.linalg import dsolve  # xpd: deprecated-in-scipy
    from scipy.sparse.linalg.dsolve import linsolve  # xpd: deprecated-in-scipy
    gone = scipy.sparse.linalg.dsolve.linsolve  # xpd: deprecated-in-scipy
    return dsolve, linsolve, gone

@case
def private_sparse_base():
    old = scipy.sparse.base.SparseEfficiencyWarning  # xpd: deprecated-in-scipy
    now = scipy.sparse.SparseEfficiencyWarning
    return old, now, SparseEfficiencyWarning_sparse_base, SparseEfficiencyWarning_sparse_base_now


def private_sparse_base_other_forms():
    from scipy.sparse import base  # xpd: deprecated-in-scipy
    from scipy.sparse.sparsetools import coo_todense  # xpd: deprecated-in-scipy
    gone = scipy.sparse.sparsetools.coo_todense  # xpd: deprecated-in-scipy
    return base, coo_todense, gone

@case
def private_spatial_transform_rotation():
    old = scipy.spatial.transform.rotation.Rotation  # xpd: deprecated-in-scipy
    now = scipy.spatial.transform.Rotation
    return old, now, Rotation_spatial_transform_rotation, Rotation_spatial_transform_rotation_now


def private_spatial_transform_rotation_other_forms():
    from scipy.spatial.transform import rotation  # xpd: deprecated-in-scipy
    from scipy.spatial.transform.rotation import no_such_name  # xpd: deprecated-in-scipy
    gone = scipy.spatial.transform.rotation.no_such_name  # xpd: deprecated-in-scipy
    return rotation, no_such_name, gone

@case
def private_spatial_ckdtree():
    old = scipy.spatial.ckdtree.cKDTree  # xpd: deprecated-in-scipy
    now = scipy.spatial.cKDTree
    return old, now, cKDTree_spatial_ckdtree, cKDTree_spatial_ckdtree_now


def private_spatial_ckdtree_other_forms():
    from scipy.spatial import ckdtree  # xpd: deprecated-in-scipy
    from scipy.spatial.ckdtree import no_such_name  # xpd: deprecated-in-scipy
    gone = scipy.spatial.ckdtree.no_such_name  # xpd: deprecated-in-scipy
    return ckdtree, no_such_name, gone

@case
def private_special_basic():
    old = scipy.special.basic.ai_zeros  # xpd: deprecated-in-scipy
    now = scipy.special.ai_zeros
    return old, now, ai_zeros_special_basic, ai_zeros_special_basic_now


def private_special_basic_other_forms():
    from scipy.special import basic  # xpd: deprecated-in-scipy
    from scipy.special.specfun import pbdv  # xpd: deprecated-in-scipy
    gone = scipy.special.specfun.pbdv  # xpd: deprecated-in-scipy
    return basic, pbdv, gone

@case
def private_stats_kde():
    old = scipy.stats.kde.gaussian_kde  # xpd: deprecated-in-scipy
    now = scipy.stats.gaussian_kde
    return old, now, gaussian_kde_stats_kde, gaussian_kde_stats_kde_now


def private_stats_kde_other_forms():
    from scipy.stats import kde  # xpd: deprecated-in-scipy
    from scipy.stats.kde import mvn  # xpd: deprecated-in-scipy
    gone = scipy.stats.kde.mvn  # xpd: deprecated-in-scipy
    return kde, mvn, gone

@case
def private_stats_mstats_basic():
    old = scipy.stats.mstats_basic.argstoarray  # xpd: deprecated-in-scipy
    now = scipy.stats.mstats.argstoarray
    return old, now, argstoarray_stats_mstats_basic, argstoarray_stats_mstats_basic_now


def private_stats_mstats_basic_other_forms():
    from scipy.stats import mstats_basic  # xpd: deprecated-in-scipy
    from scipy.stats.mstats_basic import no_such_name  # xpd: deprecated-in-scipy
    gone = scipy.stats.mstats_basic.no_such_name  # xpd: deprecated-in-scipy
    return mstats_basic, no_such_name, gone

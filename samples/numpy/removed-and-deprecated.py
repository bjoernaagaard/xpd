"""NumPy names, methods, modules and keywords that raise or warn on NumPy 2.5.

A line that xpd reports ends with a comment that names the rule. A form that raises on NumPy
2.5 sits in a plain function, which the checker never calls. Each case returns what the code
computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import xml.etree.ElementTree as ET

import numpy as np
from numpy import exceptions, isin
from numpy.lib import NumpyVersion, stride_tricks
from numpy.linalg import solve
import numpy.lib.recfunctions as rfn
from numpy.testing import assert_array_equal

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def imports():
    from numpy import float_, isin  # xpd: removed-in-numpy-2
    from numpy import isin, float_  # xpd: removed-in-numpy-2
    from numpy import AxisError as AxisProblem  # xpd: removed-in-numpy-2
    from numpy import trapz  # xpd: removed-in-numpy-2
    from numpy.lib import pad, median  # xpd: removed-in-numpy-2
    from numpy.lib import pad, NumpyVersion  # xpd: removed-in-numpy-2
    from numpy.linalg.linalg import norm  # xpd: removed-in-numpy-2
    import numpy.distutils  # xpd: removed-in-numpy-2
    import numpy.lib.function_base as fb  # xpd: removed-in-numpy-2
    from numpy.testing import dec  # xpd: removed-in-numpy-2
    np.float_ = np.float64


legacy_types = {"float": float, "int": int}


def builtin_aliases(values):
    as_int = values.astype(np.int)  # xpd: removed-in-numpy-2
    as_float = np.array(values, dtype=np.float)  # xpd: removed-in-numpy-2
    as_object = values.astype(numpy_object())
    kind = np.str  # xpd: removed-in-numpy-2
    still_there = (np.bool, np.long, np.int64, np.float64, np.str_)
    return as_int, as_float, as_object, kind, still_there


def numpy_object():
    return np.object  # xpd: removed-in-numpy-2


def scalar_aliases(values):
    a = values.astype(np.float_)  # xpd: removed-in-numpy-2
    b = np.complex_(1)  # xpd: removed-in-numpy-2
    c = np.zeros(3, dtype=np.unicode_)  # xpd: removed-in-numpy-2
    d = np.bool8  # xpd: removed-in-numpy-2
    e = np.int0  # xpd: removed-in-numpy-2
    f = np.longcomplex  # xpd: removed-in-numpy-2
    kept = (np.float64, np.complex128, np.str_, np.bool_, np.intp, np.clongdouble)
    return a, b, c, d, e, f, kept


def constants(values):
    missing = np.where(values > 0, values, np.NaN)  # xpd: removed-in-numpy-2
    top = np.Inf  # xpd: removed-in-numpy-2
    low = values.clip(np.NINF, np.PINF)  # xpd: removed-in-numpy-2, removed-in-numpy-2
    flipped = np.NINF ** 2  # xpd: removed-in-numpy-2
    zero = np.NZERO  # xpd: removed-in-numpy-2
    kept = (np.nan, np.inf, -np.inf)
    return missing, top, low, flipped, zero, kept


def renamed(values, mask):
    total = np.product(values)  # xpd: removed-in-numpy-2
    running = np.cumproduct(values, axis=0)  # xpd: removed-in-numpy-2
    some = np.sometrue(mask)  # xpd: removed-in-numpy-2
    every = np.alltrue(mask, axis=0)  # xpd: removed-in-numpy-2
    rounded = np.round_(values, 2)  # xpd: removed-in-numpy-2
    stacked = np.row_stack([values, values])  # xpd: removed-in-numpy-2
    area = np.trapz(values, dx=0.5)  # xpd: removed-in-numpy-2
    ordered = np.msort(values)  # xpd: removed-in-numpy-2
    first = np.asscalar(values[:1])  # xpd: removed-in-numpy-2
    summed = np.asscalar(values.sum())  # xpd: removed-in-numpy-2
    found = np.in1d(values, [1.0, 2.0])  # xpd: removed-in-numpy-2
    kept = (np.prod(values), np.cumprod(values), np.any(mask), np.all(mask), np.round(values, 2))
    kept_too = (np.vstack([values, values]), np.trapezoid(values), np.sort(values, axis=0), isin(values, [1.0]))
    return total, running, some, every, rounded, stacked, area, ordered, first, summed, found, kept, kept_too


def moved(values):
    try:
        np.zeros(3).sum(axis=4)
    except np.AxisError:  # xpd: removed-in-numpy-2
        pass
    except np.exceptions.AxisError:
        pass
    warning = np.RankWarning  # xpd: removed-in-numpy-2
    bounds = np.byte_bounds(values)  # xpd: removed-in-numpy-2
    padded = np.lib.pad(values, 1)  # xpd: removed-in-numpy-2
    middle = np.lib.median(values)  # xpd: removed-in-numpy-2
    kept = (np.lib.NumpyVersion("2.0.0"), np.lib.stride_tricks, np.lib.array_utils.byte_bounds(values))
    return warning, bounds, padded, middle, kept, exceptions


def without_replacement(values):
    as_float = np.asfarray(values)  # xpd: removed-in-numpy-2
    common = np.find_common_type([np.float32], [])  # xpd: removed-in-numpy-2
    np.set_string_function(None)  # xpd: removed-in-numpy-2
    taken = np.take(values, [9], mode=np.CLIP)  # xpd: removed-in-numpy-2
    size = np.MAXDIMS  # xpd: removed-in-numpy-2
    factorial = np.math.factorial(5)  # xpd: removed-in-numpy-2
    kept = (np.take(values, [9], mode="clip"), np.result_type(np.float32), np.asarray(values, dtype=np.float64))
    return as_float, common, taken, size, factorial, kept


def modules(values):
    left = np.linalg.linalg.norm(values)  # xpd: removed-in-numpy-2
    shifted = np.fft.helper.fftshift(values)  # xpd: removed-in-numpy-2
    kept = (np.linalg.norm(values), np.fft.fftshift(values))
    return left, shifted, kept


def methods(values, grid, root):
    spread = values.ptp()  # xpd: removed-in-numpy-2
    spread_rows = grid.ptp(axis=0)  # xpd: removed-in-numpy-2
    raw = values.tostring()  # xpd: removed-in-numpy-2
    raw_f = grid.tostring(order="F")  # xpd: removed-in-numpy-2
    grid.itemset((1, 2), 7.5)  # xpd: removed-in-numpy-2
    grid.itemset(0, 1, 2.5)  # xpd: removed-in-numpy-2
    grid.itemset(4, 1.0)  # xpd: removed-in-numpy-2
    kept = (np.ptp(values), values.tobytes(), ET.tostring(root), np.ma.ptp(np.ma.array(values)))
    grid[1, 2] = 7.5
    return spread, spread_rows, raw, raw_f, kept


def keywords(values, grid):
    flat = np.reshape(grid, newshape=-1)  # xpd: removed-in-numpy-2
    flat_f = np.reshape(grid, newshape=(-1,), order="F")  # xpd: removed-in-numpy-2
    shaped = np.reshape(newshape=(2, 3), a=values)  # xpd: removed-in-numpy-2
    kept_shape = (np.reshape(grid, -1), grid.reshape(-1))
    near = np.quantile(values, 0.3, interpolation="nearest")  # xpd: removed-in-numpy-2
    low = np.nanpercentile(values, 30, interpolation="lower")  # xpd: removed-in-numpy-2
    kept_quantile = np.quantile(values, 0.3, method="nearest")
    np.save("/tmp/values.npy", values, fix_imports=True)  # xpd: removed-in-numpy-2
    corr = np.corrcoef(grid, bias=1)  # xpd: removed-in-numpy-2
    masked_corr = np.ma.corrcoef(grid, ddof=0)  # xpd: removed-in-numpy-2
    text = np.array2string(values, style=repr)  # xpd: removed-in-numpy-2
    kept_save = (np.load("/tmp/values.npy", fix_imports=True), np.corrcoef(grid, rowvar=False))
    density = np.histogram(values, bins=3, normed=True)  # xpd: removed-in-numpy-2
    counts = np.bincount([1, 2], minlength=None)  # xpd: removed-in-numpy-2
    kept_counts = (np.histogram(values, density=True), np.bincount([1, 2], minlength=0))
    view = np.array([1, 2], copy=False)  # xpd: removed-in-numpy-2
    view32 = np.array(values, dtype=np.float32, copy=False)  # xpd: removed-in-numpy-2
    view2d = np.array(values, copy=False, ndmin=2)  # xpd: removed-in-numpy-2
    kept_view = (np.asarray([1, 2]), np.array(values, copy=None), values.astype(np.float64, copy=False))
    names = np.zeros(3, dtype="a5")  # xpd: removed-in-numpy-2
    codes = np.dtype([("a", "f8"), ("label", "a10")])  # xpd: removed-in-numpy-2
    as_bytes = values.astype("a")  # xpd: removed-in-numpy-2
    kept_dtype = (np.zeros(3, dtype="S5"), np.dtype([("a", "f8")]), {"a": 1}, np.array(["a"]))
    raw = np.fromstring(values.tobytes(), dtype=np.float64)  # xpd: removed-in-numpy-2
    parsed = np.fromstring("1 2", sep=" ")
    return flat, flat_f, shaped, kept_shape, near, low, kept_quantile, corr, text, kept_save, density, counts, kept_counts, view, view32, view2d, kept_view, names, codes, as_bytes, kept_dtype, raw, parsed


def deprecated_imports():
    from numpy.core.numeric import normalize_axis_tuple  # xpd: deprecated-in-numpy-2
    from numpy.core import records  # xpd: deprecated-in-numpy-2
    import numpy.core.multiarray as multiarray  # xpd: deprecated-in-numpy-2
    from numpy import core  # xpd: deprecated-in-numpy-2
    from numpy import isin, core  # xpd: deprecated-in-numpy-2
    from numpy.core.multiarray import _reconstruct
    from numpy.lib.array_utils import normalize_axis_index
    from numpy.char import chararray  # xpd: deprecated-in-numpy-2
    from numpy.char import upper
    from numpy.testing import assert_warns  # xpd: deprecated-in-numpy-2
    from numpy.testing import assert_allclose
    return normalize_axis_tuple, records, multiarray, core, _reconstruct, normalize_axis_index, chararray, upper, assert_warns, assert_allclose


def deprecated_names(values, frame):
    table = np.core.records.fromarrays([values])  # xpd: deprecated-in-numpy-2
    is_rec = isinstance(frame, np.core.recarray)  # xpd: deprecated-in-numpy-2
    close = np.core.numeric.isclose(values, values)  # xpd: deprecated-in-numpy-2
    name = np.typename("d")  # xpd: deprecated-in-numpy-2
    labels = np.char.array(["a", "b"])  # xpd: deprecated-in-numpy-2
    with np.testing.suppress_warnings() as sup:  # xpd: deprecated-in-numpy-2
        pass
    kept = (np.rec.fromarrays([values]), np.dtype("d").name, np.array(["a", "b"]), np.char.upper(["a"]), np.strings.upper(np.array(["a"])))  # xpd: char-module
    return table, is_rec, close, name, labels, sup, kept


@case
def fix_to_trunc():
    values = np.array([-2.5, -0.5, 0.5, 2.5, np.nan])
    return np.fix(values), np.trunc(values)  # xpd: deprecated-in-numpy-2


@case
def fix_on_ints():
    return np.fix(np.arange(-3, 4))  # xpd: deprecated-in-numpy-2


@case
def masked_round():
    return np.ma.round_(np.ma.array([1.25, 2.5], mask=[0, 1]), 1)  # xpd: deprecated-in-numpy-2


@case
def maximum_into_out():
    values, out = np.array([1.0, 5.0, -2.0]), np.empty(3)
    np.maximum(values, 0.0, out)  # xpd: deprecated-in-numpy-2
    low = np.minimum(values, 1, np.empty(3))  # xpd: deprecated-in-numpy-2
    kept = (np.maximum(values, 0.0, out=np.empty(3)), np.add(values, 1.0, np.empty(3)), np.maximum(values, 0.0))
    return out, low, kept


@case
def random_integers_with_size():
    state = np.random.RandomState(7)  # xpd: legacy-random
    draws = state.random_integers(1, 6, 20)  # xpd: deprecated-in-numpy-2
    more = state.random_integers(1, 6, size=(2, 3))  # xpd: deprecated-in-numpy-2
    kept = state.randint(1, 7, 5)
    return draws, more, kept, state.rand()


def random_integers_without_size(state, low, high):
    one = state.random_integers(1, 6)  # xpd: deprecated-in-numpy-2
    upto = np.random.random_integers(6)  # xpd: deprecated-in-numpy-2
    spread = state.random_integers(low, high - 1, 3)  # xpd: deprecated-in-numpy-2
    return one, upto, spread


@case
def timedelta_units():
    step = np.timedelta64(5, "s")
    missing = np.timedelta64("NaT", "ns")
    times = np.zeros(2, dtype="m8[s]")
    return step, missing, times


def timedelta_generic():
    step = np.timedelta64(5)  # xpd: deprecated-in-numpy-2
    missing = np.timedelta64("NaT")  # xpd: deprecated-in-numpy-2
    times = np.zeros(2, dtype="m8")  # xpd: deprecated-in-numpy-2
    return step, missing, times


@case
def utc_strings():
    start = np.datetime64("2024-03-01T12:00Z")  # xpd: deprecated-in-numpy-2
    stamps = np.datetime64("2024-03-01T12:00:00.250Z", "ms")  # xpd: deprecated-in-numpy-2
    plain = np.datetime64("2024-03-01T12:00")
    return start, stamps, plain


def offset_strings():
    return np.datetime64("2024-03-01T12:00+01:00")  # xpd: deprecated-in-numpy-2


def fft_shapes(grid):
    padded = np.fft.fftn(grid, s=(4, 4))  # xpd: deprecated-in-numpy-2
    real = np.fft.rfftn(grid, (4, 4))  # xpd: deprecated-in-numpy-2
    kept = (np.fft.fftn(grid, s=(4, 4), axes=(0, 1)), np.fft.fft2(grid, s=(4, 4)), np.fft.fftn(grid))
    return padded, real, kept


def set_attributes(shape):
    grid = np.arange(6.0)
    grid.shape = (2, 3)  # xpd: deprecated-in-numpy-2
    codes = np.zeros(4, dtype=np.int32)
    codes.dtype = np.float32  # xpd: deprecated-in-numpy-2
    window = np.arange(5)
    window.strides = (8,)  # xpd: deprecated-in-numpy-2
    shape.shape = (1,)
    return grid, codes, window


class Holder:
    def __init__(self, shape):
        self.shape = shape
        self.dtype = np.float64


def compatible(values):
    if np.__version__ < "2":
        kind = np.float_
    else:
        kind = np.float64
    if hasattr(np, "bool8"):
        flag = np.bool8
    warning = np.ComplexWarning if NP_LT_2 else np.exceptions.ComplexWarning
    try:
        from numpy import AxisError
    except ImportError:
        from numpy.exceptions import AxisError
    try:
        from numpy.lib.array_utils import normalize_axis_tuple
    except ModuleNotFoundError:
        from numpy.core.numeric import normalize_axis_tuple
    return kind, flag, warning, AxisError, normalize_axis_tuple


NP_LT_2 = False


def copy_keyword(values):
    copied = np.array(values, copy=True)
    same = np.array(values, copy=False)
    try:
        import pytest
    except ModuleNotFoundError:
        return copied, same
    with pytest.raises(ValueError):
        np.array([1, 2], copy=False)
    return copied, same

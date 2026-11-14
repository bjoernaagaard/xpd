"""Older NumPy habits that still run on NumPy 2.5, next to the API that NumPy's docs prefer.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import random
import time

import numpy as np
import numpy.testing as npt
from numpy.random import default_rng
from numpy.random import SeedSequence, rand as legacy_rand  # xpd: legacy-random
from numpy.testing import assert_, assert_raises
from numpy.random import randn, seed  # xpd: legacy-random

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def legacy_random():
    np.random.seed(0)  # xpd: legacy-random
    a = np.random.rand(3, 2)  # xpd: legacy-random
    b = np.random.randint(0, 5, 3)  # xpd: legacy-random
    c = numpy_random_sample()
    d = np.random.randn(4)  # xpd: legacy-random
    e = np.random.choice(10, 3, replace=False)  # xpd: legacy-random
    f = np.random.choice(10, 3)  # xpd: legacy-random
    g = np.random.exponential(2.0, 5)  # xpd: legacy-random
    h = np.random.normal(0.0, 1.0, 5)  # xpd: legacy-random
    state = np.random.get_state()  # xpd: legacy-random
    rs = np.random.RandomState(4)  # xpd: legacy-random
    return a, b, c, d, e, f, g, h, state, rs, seed, randn


def numpy_random_sample():
    return np.random.random_sample(3)  # xpd: legacy-random


@case
def generator_forms():
    rng = np.random.default_rng(0)
    rng2 = default_rng(1)
    random.seed(3)
    stdlib = [random.random(), random.choice([1, 2, 3])]
    return rng.random((3, 2)), rng.integers(0, 5, 3), rng2.choice(10, 3, replace=False), stdlib


def generator_per_pass():
    draws = []
    for _ in range(3):
        rng = np.random.default_rng()  # xpd: rng-in-loop
        draws.append(rng.random())
    draws += [np.random.default_rng().random() for _ in range(3)]  # xpd: rng-in-loop
    return draws


@case
def generator_per_seed():
    per_seed = [np.random.default_rng(s).random() for s in range(3)]
    out = []
    for value in np.random.default_rng(5).random(3):
        out.append(value)
    rng = np.random.default_rng(7)
    for _ in range(3):
        out.append(rng.random())
    return per_seed, out


def clock_seeds():
    np.random.seed(int(time.time()))  # xpd: clock-seed, legacy-random
    rng = np.random.default_rng(int(time.time() * 1000))  # xpd: clock-seed
    seq = np.random.SeedSequence(time.time_ns())  # xpd: clock-seed
    start = time.time()
    fixed = np.random.default_rng(12345)
    return rng, seq, start, fixed


@case
def legacy_polynomial():
    coef = [1.0, -3.0, 2.0]
    x = np.linspace(0.0, 1.0, 5)
    fit = np.polyfit(x, 2 * x + 1, 1)  # xpd: legacy-polynomial
    value = np.polyval(coef, 0.5)  # xpd: legacy-polynomial
    zeros = np.roots(coef)  # xpd: legacy-polynomial
    p = np.poly1d(coef)  # xpd: legacy-polynomial
    from_roots = np.poly([1.0, 2.0])  # xpd: legacy-polynomial
    slope = np.polyder(coef)  # xpd: legacy-polynomial
    product = np.polymul(coef, coef)  # xpd: legacy-polynomial
    return fit, value, zeros, p(2.0), from_roots, slope, product


@case
def new_polynomial():
    P = np.polynomial.Polynomial
    x = np.linspace(0.0, 1.0, 5)
    fit = P.fit(x, 2 * x + 1, 1).convert().coef
    value = np.polynomial.polynomial.polyval(0.5, [2.0, -3.0, 1.0])
    return fit, value, P([2.0, -3.0, 1.0]).roots(), P.fromroots([1.0, 2.0]).coef


def almost_equal_asserts():
    a = np.array([1.0, 2.0])
    b = a + 1e-9
    np.testing.assert_almost_equal(a, b)  # xpd: almost-equal-assert
    np.testing.assert_almost_equal(a, b, 6)  # xpd: almost-equal-assert
    npt.assert_array_almost_equal(a, b)  # xpd: almost-equal-assert
    npt.assert_array_almost_equal(a, b, decimal=5)  # xpd: almost-equal-assert
    npt.assert_array_almost_equal(a, b, err_msg="drift")  # xpd: almost-equal-assert
    npt.assert_approx_equal(1.0, 1.0 + 1e-9)  # xpd: almost-equal-assert
    np.testing.assert_allclose(a, b, rtol=0, atol=1.5e-7)
    npt.assert_array_equal(a, a)


def asserts_on_arrays():
    a = np.array([1.0, 2.0])
    b = a.copy()
    assert np.allclose(a, b)  # xpd: assert-on-arrays
    assert np.allclose(a, b, atol=1e-3)  # xpd: assert-on-arrays
    assert np.array_equal(a, b)  # xpd: assert-on-arrays
    assert np.all(a == b)  # xpd: assert-on-arrays
    assert (a == b).all()  # xpd: assert-on-arrays
    assert (a == b).all(), "a and b differ"  # xpd: assert-on-arrays
    assert not np.allclose(a, b + 1)
    assert np.isclose(a, b).all()
    assert a.shape == b.shape


@case
def char_module():
    words = np.array(["ab c", " x ", "Tab\tend"])
    upper = np.char.upper(words)  # xpd: char-module
    lengths = np.char.str_len(words)  # xpd: char-module
    joined = np.char.add(words, "!")  # xpd: char-module
    same = np.char.equal(words, "ab c")  # xpd: char-module
    twice = np.char.multiply(words, 2)  # xpd: char-module
    parts = np.char.partition(words, " ")  # xpd: char-module
    pieces = np.char.split(words)
    return upper, lengths, joined, same, twice, parts, pieces, np.strings.lower(words)


@case
def dot_on_matrices():
    a = np.arange(6.0).reshape(2, 3)
    b = np.arange(6.0).reshape(3, 2)
    v = np.arange(3.0)
    m = np.dot(a, b)  # xpd: dot-on-matrices
    mv = np.dot(a, v)  # xpd: dot-on-matrices
    shifted = np.dot(a + 1, b)  # xpd: dot-on-matrices
    return m, mv, shifted, a @ b, np.vdot(v, v), np.inner(v, v)


@case
def mergesort_kind():
    x = np.array([3, 1, 2, 1, 3, 2, 1])
    order = np.argsort(x, kind="mergesort")  # xpd: mergesort-kind
    y = x.copy()
    y.sort(kind='mergesort')  # xpd: mergesort-kind
    stable = np.argsort(x, kind="stable")
    quick = np.sort(x, kind="quicksort")
    return order, y, stable, quick


@case
def printoptions_restore():
    x = np.array([1 / 3, 2 / 3])
    saved = np.get_printoptions()
    np.set_printoptions(precision=2)
    short = repr(x)
    np.set_printoptions(**saved)  # xpd: printoptions-restore
    with np.printoptions(precision=3):
        scoped = repr(x)
    return short, scoped, repr(x)


@case
def strided_views():
    x = np.arange(8.0)
    windows = np.lib.stride_tricks.as_strided(x, shape=(6, 3), strides=(8, 8))  # xpd: as-strided
    safe = np.lib.stride_tricks.sliding_window_view(x, 3)
    repeated = np.broadcast_to(x, (2, 8))
    return windows.copy(), safe, repeated


def other_aliases():
    import numpy as nmp  # xpd: numpy-import-alias
    import numpy as xp
    from numpy import linalg as la

    return nmp, xp, la


if False:
    from numpy import *  # xpd: numpy-import-alias


@case
def sum_of_products():
    a = np.arange(12.0).reshape(4, 3)
    b = np.ones((4, 3))
    z = a + 1j
    dots = np.sum(a * b, axis=-1)  # xpd: sum-of-products
    cdots = np.sum(z.conj() * b, axis=-1)  # xpd: sum-of-products
    columns = np.sum(a * b, axis=0)
    scaled = np.sum(np.sum(a, axis=-1) * 2.0, -1)
    return dots, cdots, columns, scaled, np.vecdot(a, b)


@case
def bit_counts():
    x = np.array([3, 255, 1024], dtype=np.uint64)
    total = np.unpackbits(x.view(np.uint8)).sum()  # xpd: unpackbits-popcount
    each = np.sum(np.unpackbits(x.view(np.uint8).reshape(-1, 8), axis=1), axis=1)  # xpd: unpackbits-popcount
    bits = np.unpackbits(x.view(np.uint8))
    return total, each, bits, np.bitwise_count(x)


def legacy_testing_helpers():
    a = np.ones(2)
    assert_(a.shape == (2,))  # xpd: legacy-testing-helper
    np.testing.assert_(a.sum() == 2, "sum is off")  # xpd: legacy-testing-helper
    assert_raises(ValueError, int, "x")  # xpd: legacy-testing-helper
    with np.testing.assert_raises(ValueError):  # xpd: legacy-testing-helper
        int("x")
    assert a.shape == (2,)
    ok = assert_(True)
    return ok

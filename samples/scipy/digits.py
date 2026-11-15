"""SciPy forms that lose digits, next to the forms that keep them.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import math

import numpy as np
import scipy as sp
import scipy.special as sc
from scipy import linalg, special, stats
from scipy.linalg import det
from scipy.special import erf, expit, factorial, ndtri
from scipy.stats import binom, chi2, norm

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def log_of_special():
    x = np.array([-40.0, -1.0, 3.0])
    v = np.array([0.0, -50.0, -800.0])
    n = np.array([5.0, 200.0])
    return (
        np.log(special.ndtr(x)),  # xpd: log-of-special
        special.log_ndtr(x),
        np.log(sc.gamma(x + 41)),  # xpd: log-of-special
        np.log(sc.gamma(x + 41) + 1),
        np.log(sp.special.factorial(n)),  # xpd: log-of-special
        np.log(special.factorial(n - 1)),  # xpd: log-of-special
        np.log(special.factorial(5, exact=True)),
        np.log(special.beta(x + 41, 1e6)),  # xpd: log-of-special
        special.betaln(x + 41, 1e6),
        np.log(special.expit(x)),  # xpd: log-of-special
        np.log1p(special.expit(x)),
        np.log(special.softmax(v, axis=0)),  # xpd: log-of-special
        special.log_softmax(v),
        np.log(expit(x)),  # xpd: log-of-special
        np.log(np.exp(x)),
    )


@case
def log_of_dist_method():
    x = np.array([-40.0, 0.5, 40.0])
    k = np.array([0, 3, 400])
    return (
        np.log(stats.norm.pdf(x, loc=1.0)),  # xpd: log-of-dist-method
        np.log(chi2.pdf(x + 41, 3)),  # xpd: log-of-dist-method
        np.log(stats.poisson(3.0).pmf(k)),  # xpd: log-of-dist-method
        np.log(binom.pmf(k, 1000, 0.2)),
        stats.norm.logpdf(x),
        np.log(stats.t.cdf(x, 5)),  # xpd: log-of-dist-method
        np.log(stats.f.cdf(x + 41, 3, 4)),
        np.log(norm.sf(x)),  # xpd: log-of-dist-method
        np.log(stats.beta.sf(0.5, 2, 3)),
    )


@case
def cdf_complement():
    z = np.array([-2.0, 5.0, 9.0])
    return (
        1 - stats.norm.cdf(z),  # xpd: cdf-complement
        2 * (1 - norm.cdf(np.abs(z))),  # xpd: cdf-complement
        1.0 - stats.t.cdf(z, 30),  # xpd: cdf-complement
        1 - binom.cdf(3, 10, 0.2),
        stats.norm.sf(z),
        1 - stats.chi2.sf(z + 3, 3),  # xpd: cdf-complement
        1 - stats.beta.sf(0.5, 2, 3),
        2 - stats.norm.cdf(z),
    )


@case
def special_complement():
    x = np.array([0.5, 4.0, 6.0])
    return (
        1 - special.erf(x),  # xpd: special-complement
        1 - sc.gammainc(2.0, x * 10),  # xpd: special-complement
        1 - special.erfcx(x),
        2 - special.erf(x),
        1 - special.betainc(2.0, 30.0, x / 7),  # xpd: special-complement
        special.betaincc(2.0, 30.0, x / 7),
        0.5 * (1 + special.erf(x / np.sqrt(2))),  # xpd: special-complement
        1 + special.erf(x - 1),
        1 - special.ndtr(x),  # xpd: special-complement
        1 - expit(x * 10),  # xpd: special-complement
        1 - special.ndtr(x - 1),
        1 - erf(x),  # xpd: special-complement
        1 - math.erf(0.5),
    )


@case
def ppf_of_complement():
    p = np.array([1e-20, 1e-10, 0.025])
    alpha = 0.05
    return (
        stats.norm.ppf(1 - alpha / 2),  # xpd: ppf-of-complement
        stats.t.ppf(1 - p, 10),  # xpd: ppf-of-complement
        stats.binom.ppf(1 - p, 10, 0.5),
        stats.norm.ppf(1 - 0.05),
        stats.norm.ppf(p),
        stats.norm.isf(1.0 - p),  # xpd: ppf-of-complement
        stats.pareto.isf(1 - p, 2.5),
        special.ndtri(1 - p),  # xpd: ppf-of-complement
        ndtri(1. - p) ** 2,  # xpd: ppf-of-complement
        special.ndtri(p),
    )


@case
def comb_by_factorials():
    n, k = 200.0, 3.0
    return (
        special.factorial(n) / (special.factorial(k) * special.factorial(n - k)),  # xpd: comb-by-factorials
        special.factorial(n) / special.factorial(k),
        factorial(n) / factorial(k) / factorial(n - k),  # xpd: comb-by-factorials
        special.comb(n, k),
    )


@case
def log_of_scipy_det():
    r = np.random.default_rng(0)
    c = np.cov(r.standard_normal((360, 120)) * 1e-3, rowvar=False)
    return (
        np.log(linalg.det(c)),  # xpd: log-of-scipy-det
        np.log(np.abs(det(c))),  # xpd: log-of-scipy-det
        np.linalg.slogdet(c)[1],
        np.log(linalg.norm(c)),
    )

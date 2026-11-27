"""Indicators and decision making with arguments that give another number than the code means.

A line that xpd reports ends with a comment that names the rule. A form that raises on pymoo
0.6.2 sits in a function that is not a case and that nothing calls. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from pymoo.decomposition.asf import ASF
from pymoo.indicators.gd import GD
from pymoo.indicators.hv import HV, Hypervolume
from pymoo.indicators.igd import IGD
from pymoo.mcdm.pseudo_weights import PseudoWeights
from pymoo.problems import get_problem

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


def front():
    return np.array([[0.1, 0.9], [0.3, 0.6], [0.5, 0.5], [0.6, 0.3], [0.9, 0.1]])


@case
def reference_point_of_the_hypervolume():
    F = front()
    moving = HV(ref_point=F.max(axis=0))(F)  # xpd: hv-reference-point
    with_numpy = Hypervolume(ref_point=np.max(F, axis=0))(F)  # xpd: hv-reference-point
    fixed = HV(ref_point=np.array([1.0, 1.0]))(F)
    beyond = HV(ref_point=F.max(axis=0) * 1.1)(F)
    worst = F.max(axis=0)
    named = HV(ref_point=worst)(F)
    return moving, with_numpy, fixed, beyond, named


def hypervolume_without_a_point():
    return HV()(front())  # xpd: hv-reference-point


@case
def distance_on_a_stretched_front():
    pf = get_problem("zdt1").pareto_front() * np.array([1000.0, 1.0])
    F = pf[::10] + np.array([50.0, 0.05])
    ignored = IGD(pf, normalize=True)(F)  # xpd: indicator-normalize-ignored
    plain = IGD(pf)(F)
    scaled = IGD(pf, zero_to_one=True)(F)
    off = GD(pf, normalize=False)(F)
    return ignored, plain, scaled, off, normalize(F)


def normalize(F, normalize=True):  # xpd: generic-verb-name
    return (F - F.min(axis=0)) / (F.max(axis=0) - F.min(axis=0)) if normalize else F


@case
def one_solution_by_weights():
    F = front()
    weights = np.array([0.2, 0.8])
    as_given = ASF().do(F, np.array([0.2, 0.8])).argmin()  # xpd: decision-weights
    inverted = ASF().do(F, 1 / weights).argmin()
    equal = ASF().do(F, np.array([0.5, 0.5])).argmin()
    by_name = ASF().do(F, weights).argmin()
    shares = PseudoWeights(np.array([0.2, 0.8])).do(F)
    counts = PseudoWeights(np.array([2, 8])).do(F)  # xpd: decision-weights, decision-weights
    percent = PseudoWeights([20.0, 80.0]).do(F)  # xpd: decision-weights, decision-weights
    whole = PseudoWeights([1.0, 0.0]).do(F)
    return as_given, inverted, equal, by_name, shares, counts, percent, whole

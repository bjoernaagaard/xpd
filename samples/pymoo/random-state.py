"""Runs that draw random numbers without a seed, operators that draw outside pymoo's generator, and the seeded forms.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.core.mutation import Mutation
from pymoo.core.sampling import Sampling
from pymoo.optimize import minimize
from pymoo.problems import get_problem

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def runs_without_a_seed():
    problem = get_problem("zdt1")
    bare = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5))  # xpd: unseeded-run
    plain = NSGA2(pop_size=20)
    named = minimize(problem, plain, termination=("n_gen", 5), verbose=False)  # xpd: unseeded-run
    return bare.F.shape[1], named.F.shape[1]


@case
def runs_with_a_seed():
    problem = get_problem("zdt1")
    seeded = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), seed=1)
    on_the_algorithm = minimize(problem, NSGA2(pop_size=20, seed=1), ("n_gen", 5))
    algorithm = NSGA2(pop_size=20, seed=1)
    by_name = minimize(problem, algorithm, ("n_gen", 5))
    options = {"seed": 1}
    passed_on = minimize(problem, NSGA2(pop_size=20), ("n_gen", 5), **options)
    return seeded.F, on_the_algorithm.F, by_name.F, passed_on.F


class GlobalSampling(Sampling):
    def _do(self, problem, n_samples, **kwargs):
        return np.random.random((n_samples, problem.n_var))  # xpd: numpy-seed-ignored, legacy-random


class GlobalMutation(Mutation):
    def _do(self, problem, X, **kwargs):
        return np.clip(X + np.random.normal(0, 0.1, X.shape), 0, 1)  # xpd: numpy-seed-ignored, legacy-random


class SeededSampling(Sampling):
    def _do(self, problem, n_samples, random_state=None, **kwargs):
        return random_state.random((n_samples, problem.n_var))


class OwnGenerator(Sampling):
    def __init__(self):
        super().__init__()
        self.rng = np.random.default_rng(3)

    def _do(self, problem, n_samples, **kwargs):
        return self.rng.random((n_samples, problem.n_var))


class Helper:
    def _do(self, n):
        return np.random.random(n)  # xpd: legacy-random


@case
def operators_and_the_seed():
    problem = get_problem("zdt1")
    unseeded = minimize(problem, NSGA2(pop_size=20, sampling=GlobalSampling(), mutation=GlobalMutation()), ("n_gen", 3), seed=1)
    a = minimize(problem, NSGA2(pop_size=20, sampling=SeededSampling()), ("n_gen", 3), seed=1)
    b = minimize(problem, NSGA2(pop_size=20, sampling=SeededSampling()), ("n_gen", 3), seed=1)
    own = minimize(problem, NSGA2(pop_size=20, sampling=OwnGenerator()), ("n_gen", 3), seed=1)
    return unseeded.F.shape[1], a.F, b.F, own.F, Helper()._do(2).shape

"""Problem classes whose _evaluate gives pymoo wrong numbers or raises, and the classes that are right.

A line that xpd reports ends with a comment that names the rule. A form that raises on pymoo
0.6.2 sits in a class that no case builds. Each case returns what the code computes, so a copy
fixed with `xpd check --fix` can be compared with this file.
"""

from multiprocessing.pool import ThreadPool

import numpy as np
from pymoo.algorithms.moo.nsga2 import NSGA2
from pymoo.algorithms.soo.nonconvex.ga import GA
from pymoo.core.problem import ElementwiseProblem, Problem
from pymoo.optimize import minimize
from pymoo.parallelization import StarmapParallelization

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


class RowsPerObjective(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=2, n_ieq_constr=2, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        f1 = x[:, 0]
        f2 = 10 + x[:, 1]
        out["F"] = np.array([f1, f2])  # xpd: objectives-stacked-by-row
        out["G"] = np.vstack([0.5 - f1, f2 - 12])  # xpd: objectives-stacked-by-row


class ColumnsPerObjective(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=2, n_ieq_constr=2, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        f1 = x[:, 0]
        f2 = 10 + x[:, 1]
        out["F"] = np.column_stack([f1, f2])
        out["G"] = np.array([0.5 - f1, f2 - 12]).T
        out["seen"] = np.array([f1, f2])


class OneSolution(ElementwiseProblem):
    def __init__(self, **kwargs):
        super().__init__(n_var=2, n_obj=2, xl=0.0, xu=1.0, **kwargs)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = np.array([x[0], 10 + x[1]])


@case
def objectives_by_row_and_by_column():
    X = np.array([[0.1, 0.2], [0.3, 0.4], [0.5, 0.6], [0.7, 0.8]])
    return RowsPerObjective().evaluate(X), ColumnsPerObjective().evaluate(X), OneSolution().evaluate(X)


class ReturnsTheValue(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        return (x**2).sum(axis=1)  # xpd: evaluate-returns-value


class WritesTheValue(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        if x is None:
            return None
        out["F"] = (x**2).sum(axis=1)
        return out


class HandsOutToAHelper(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        return fill(x, out)


def fill(x, out):
    out["F"] = (x**2).sum(axis=1)


class NotAProblem:
    def _evaluate(self, x, out):
        return x.sum()


@case
def value_returned_and_value_written():
    X = np.array([[0.1, 0.2], [0.3, 0.4]])
    return ReturnsTheValue().evaluate(X), WritesTheValue().evaluate(X), HandsOutToAHelper().evaluate(X), NotAProblem()._evaluate(X, {})


class ConstraintNotCounted(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]  # xpd: constraint-not-declared
        out["H"] = x[:, 1] - 0.25  # xpd: constraint-not-declared


class ConstraintCounted(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, n_ieq_constr=1, n_eq_constr=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]
        out["H"] = x[:, 1] - 0.25


class CountFromTheCaller(Problem):
    def __init__(self, **kwargs):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0, **kwargs)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]


@case
def constraints_that_are_counted():
    X = np.array([[0.1, 0.2], [0.9, 0.25]])
    return ConstraintCounted().evaluate(X), CountFromTheCaller(n_ieq_constr=1).evaluate(X)


class OldKeywords(Problem):
    def __init__(self):
        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, elementwise_evaluation=True, type_var=int)  # xpd: ignored-problem-keyword, ignored-problem-keyword

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2 + x[2] ** 2


class OldParallel(ElementwiseProblem):
    def __init__(self, pool):
        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, parallelization=("starmap", pool.starmap))  # xpd: ignored-problem-keyword

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2 + x[2] ** 2


class CurrentKeywords(ElementwiseProblem):
    def __init__(self, **kwargs):
        super().__init__(n_var=3, n_obj=1, xl=0.0, xu=1.0, vtype=float, **kwargs)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2 + x[2] ** 2


def settings(elementwise_evaluation=True, type_var=int, n_constr=0):
    return dict(elementwise_evaluation=elementwise_evaluation, type_var=type_var, n_constr=n_constr)


@case
def old_and_current_keywords():
    X = np.array([[0.1, 0.2, 0.3], [0.4, 0.5, 0.6], [0.7, 0.8, 0.9]])
    with ThreadPool(2) as pool:
        ignored = OldParallel(pool).evaluate(X)
        runner = CurrentKeywords(elementwise_runner=StarmapParallelization(pool.starmap)).evaluate(X)
    return OldKeywords().evaluate(X), ignored, runner, settings()


class OldCount(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, n_constr=1, xl=0.0, xu=1.0)  # xpd: n-constr-keyword

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]


class BothCounts(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, n_constr=1, n_ieq_constr=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)
        out["G"] = 0.5 - x[:, 0]


@case
def constraint_count_under_its_old_name():
    res = minimize(OldCount(), GA(pop_size=20), ("n_gen", 30), seed=1)
    both = minimize(BothCounts(), GA(pop_size=20), ("n_gen", 30), seed=1)
    return res.X, res.F, res.G, OldCount().n_ieq_constr, OldCount().n_constr, both.F


class ColumnsOfOneSolution(ElementwiseProblem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[:, 0] ** 2 + x[:, 1] ** 2  # xpd: evaluate-indexing, evaluate-indexing


class VariablesOfThePopulation(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = x[0] ** 2 + x[1] ** 2  # xpd: evaluate-indexing, evaluate-indexing


class RightIndexing(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        first = x[0]
        out["F"] = x[:, 0] ** 2 + x[:, 1] ** 2 + 0 * first[0]


class RightElementwise(ElementwiseProblem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        grid = np.outer(x, x)
        out["F"] = x[0] ** 2 + x[1] ** 2 + 0 * grid[:, 0].sum()


class LoopsOverSolutions(Problem):
    def __init__(self):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = np.array([row[0] ** 2 + row[1] ** 2 for row in x])


class FlagFromTheCaller(Problem):
    def __init__(self, elementwise=True, **kwargs):
        super().__init__(n_var=2, n_obj=2, xl=0.0, xu=1.0, elementwise=elementwise, **kwargs)

    def _evaluate(self, x, out, *args, **kwargs):
        # The caller turns elementwise on, so x is one solution and a list of numbers is right.
        out["F"] = np.array([x[0], 10 + x[1]])


@case
def indexing_that_fits_the_class():
    X = np.array([[0.1, 0.2], [0.3, 0.4]])
    return RightIndexing().evaluate(X), RightElementwise().evaluate(X), LoopsOverSolutions().evaluate(X), FlagFromTheCaller().evaluate(X)


class BoundsByPosition(Problem):
    def __init__(self):
        super().__init__(2, 1, 0, np.zeros(2), np.ones(2))  # xpd: positional-problem-bounds

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)


class BoundsByKeyword(Problem):
    def __init__(self):
        super().__init__(2, 1, 0, xl=np.zeros(2), xu=np.ones(2))

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)


class CountsByPosition(Problem):
    def __init__(self, xl, xu):
        # The fourth argument is a whole number, so it is the count of equality constraints.
        super().__init__(2, 1, 0, 0, xl, xu)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)


@case
def bounds_by_position_and_by_keyword():
    shifted = BoundsByPosition()
    counted = CountsByPosition(np.zeros(2), np.ones(2))
    best = minimize(BoundsByKeyword(), GA(pop_size=10), ("n_gen", 3), seed=1).F
    return shifted.n_eq_constr, shifted.xl, shifted.xu, counted.xl, counted.xu, best


class Vectorized(Problem):
    def __init__(self, **kwargs):
        super().__init__(n_var=2, n_obj=1, xl=0.0, xu=1.0, **kwargs)

    def _evaluate(self, x, out, *args, **kwargs):
        out["F"] = (x**2).sum(axis=1)


def runner_given_wrong():
    pool = ThreadPool(2)
    bare = OneSolution(elementwise_runner=pool.starmap)  # xpd: elementwise-runner
    return minimize(bare, NSGA2(pop_size=10), ("n_gen", 3), seed=1)


@case
def runners():
    with ThreadPool(2) as pool:
        runner = StarmapParallelization(pool.starmap)
        unused = Vectorized(elementwise_runner=runner)  # xpd: elementwise-runner
        used = OneSolution(elementwise_runner=runner)
        a = minimize(unused, GA(pop_size=10), ("n_gen", 3), seed=1)
        b = minimize(used, NSGA2(pop_size=10), ("n_gen", 3), seed=1)
    return a.F, b.F

"""Changes to an existing problem and checks after solving it."""

import cvxpy as cp
from cvxpy import Minimize, Problem
from scipy.optimize import OptimizeResult


def discarded_constraints() -> Problem:
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.square(x)))
    problem.constraints.append(x >= 1)  # xpd: mutate-problem-constraints
    problem.constraints.extend([x <= 2])  # xpd: mutate-problem-constraints
    problem.constraints.insert(0, x >= 0)  # xpd: mutate-problem-constraints
    return problem


def rebuilt_problem() -> Problem:
    x = cp.Variable()
    problem = Problem(Minimize(cp.square(x)))
    constraints = problem.constraints
    constraints.append(x >= 1)
    return Problem(problem.objective, constraints)


def objective_as_status() -> None:
    problem = Problem(Minimize(0))
    if problem.solve():  # xpd: solve-result-as-status
        return
    elif not (problem.solve()):  # xpd: solve-result-as-status
        return
    assert problem.solve()  # xpd: solve-result-as-status


def check_status() -> bool:
    problem = Problem(Minimize(0))
    objective = problem.solve()
    assert problem.status == cp.OPTIMAL, problem.solve()
    return objective == 0 and problem.status == cp.OPTIMAL


def unrelated_constraint_list() -> list[float]:
    problem = cp.Problem(cp.Minimize(0))
    problem = OptimizeResult(constraints=[1.0])
    problem.constraints.append(2.0)
    return problem.constraints

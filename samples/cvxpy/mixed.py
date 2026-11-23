"""Names from other libraries and names rebound in the same file."""

from types import ModuleType

import cvxpy as cp
import numpy as np
from cvxpy import sum as vec
from scipy.optimize import OptimizeResult


def unrelated_vec() -> cp.Expression:
    return vec(cp.Variable(3))


def shadowed_identity(np: ModuleType) -> cp.Expression:
    return cp.quad_form(cp.Variable(3), np.eye(3))


def numpy_vector() -> np.ndarray:
    return np.reshape(np.ones((2, 3)), (6,))


def unrelated_solve(problem: OptimizeResult) -> bool:
    return bool(problem.success)


def parameter_named_variable() -> cp.Expression:
    from cvxpy import Parameter as Variable

    x = Variable(2)
    return cp.quad_form(x, np.eye(2))


def shadowed_bool(bool) -> None:
    problem = cp.Problem(cp.Minimize(0))
    bool(problem.solve())


def shadowed_reshape() -> np.ndarray:
    from numpy import reshape

    return reshape(np.ones((2, 3)), (6,))

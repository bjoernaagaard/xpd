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

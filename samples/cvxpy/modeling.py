"""CVXPY operators and atoms with explicit alternatives."""

import cvxpy as cvx
import numpy as np
from cvxpy import vec as vectorize
from cvxpy import Variable, reshape, vec


def column_major() -> tuple[cvx.Expression, ...]:
    matrix = Variable((2, 3))
    return (
        reshape(matrix, (6,)),  # xpd: implicit-reshape-order
        cvx.reshape(matrix, (6,), None),  # xpd: implicit-reshape-order
        cvx.reshape(expr=matrix, shape=(6,), order=None),  # xpd: implicit-reshape-order
        vec(matrix),  # xpd: implicit-reshape-order
        cvx.vec(matrix, order=None),  # xpd: implicit-reshape-order
        cvx.vec(matrix, None),  # xpd: implicit-reshape-order
        matrix.flatten(),  # xpd: implicit-reshape-order
        matrix.flatten(None),  # xpd: implicit-reshape-order
        matrix.flatten(order=None),  # xpd: implicit-reshape-order
        cvx.reshape(matrix, (6,),),  # xpd: implicit-reshape-order
        cvx.vec(matrix, None,),  # xpd: implicit-reshape-order
    )


def explicit_order() -> tuple[cvx.Expression, ...]:
    matrix = Variable((2, 3))
    options = {"order": "F"}
    return (
        cvx.reshape(matrix, (6,), "F"),
        vec(X=matrix, order="C"),
        matrix.flatten("C"),
        matrix.flatten(**options),
    )


def matrix_product() -> tuple[cvx.Expression, ...]:
    coefficients = cvx.Parameter((3, 3), value=np.eye(3))
    x = Variable(3)
    scalar = Variable()
    singleton = Variable(1)
    return (
        coefficients * x,  # xpd: star-matmul
        x * coefficients,  # xpd: star-matmul
        coefficients @ x,
        cvx.multiply(coefficients, x),
        2 * x,
        scalar * x,
        singleton * x,
    )


def identity_quadratic() -> cvx.Expression:
    x = Variable(3)
    return cvx.quad_form(x, np.eye(3))  # xpd: dense-identity-quad-form


def identity_inline() -> cvx.Expression:
    return cvx.quad_form(Variable(3), np.identity(3))  # xpd: dense-identity-quad-form


def other_quadratics() -> tuple[cvx.Expression, ...]:
    x = Variable(3)
    return (
        cvx.sum_squares(x),
        cvx.quad_form(x, np.diag([2, 3, 4])),
        cvx.quad_form(Variable(3, complex=True), np.eye(3)),
        cvx.quad_form(Variable(3), np.eye(3, dtype=float)),
    )


def numpy_order() -> np.ndarray:
    matrix = np.ones((2, 3))
    return matrix.flatten()


def renamed_vec() -> cvx.Expression:
    matrix = Variable((2, 3))
    return vectorize(matrix)  # xpd: implicit-reshape-order

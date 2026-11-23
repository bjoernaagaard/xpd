"""Canonical CVXPY symbols reached through renamed imports."""

import cvxpy as cp
from cvxpy import Parameter as Coefficients
from cvxpy import Problem as Model
from cvxpy import Variable as Decision
from cvxpy import quad_form as quadratic
from numpy import eye, identity as dense_identity


def discarded_edits() -> Model:
    x = Decision()
    problem = Model(cp.Minimize(x), [x >= 0])
    problem.constraints.clear()  # xpd: mutate-problem-constraints
    problem.constraints[:0] = [x >= 1]  # xpd: mutate-problem-constraints
    problem.constraints[0:1] += [x <= 2]  # xpd: mutate-problem-constraints
    del problem.constraints[0]  # xpd: mutate-problem-constraints
    return problem


def copied_edits() -> Model:
    x = Decision()
    problem = Model(cp.Minimize(x), [x >= 0])
    constraints = problem.constraints
    constraints.clear()
    constraints[:0] = [x >= 1]
    constraints[0:1] += [x <= 2]
    del constraints[1]
    return Model(problem.objective, constraints)


def truth_tests() -> None:
    problem = Model(cp.Minimize(0))
    if problem.solve():  # xpd: solve-result-as-status
        return
    bool((problem.solve()))  # xpd: solve-result-as-status
    result = 1 if problem.solve() else 0  # xpd: solve-result-as-status
    assert result == 0, problem.solve()
    assert problem.solve() == 0
    result if result == 0 else problem.solve()


def order_forms() -> tuple[cp.Expression, ...]:
    x = Decision((2, 3))
    return (
        x.reshape((6,)),  # xpd: implicit-reshape-order
        cp.reshape(order=None, expr=x, shape=(6,)),  # xpd: implicit-reshape-order
        cp.vec(X=x, order=(None)),  # xpd: implicit-reshape-order
        x.flatten(),  # xpd: implicit-reshape-order
        x.reshape(order=((None)), shape=(6,)),  # xpd: implicit-reshape-order
        x.reshape((6,), (None)),  # xpd: implicit-reshape-order
    )


def explicit_order_forms() -> tuple[cp.Expression, ...]:
    x = Decision((2, 3))
    return (
        x.reshape((6,), order="C"),
        cp.reshape(order="F", expr=x, shape=(6,)),
        cp.vec(order="C", X=x),
        x.flatten(order=("F")),
    )


def compatible_products() -> tuple[cp.Expression, ...]:
    x = Decision(shape=2)
    coefficients = Coefficients(shape=(3, 2))
    return (
        coefficients * x,  # xpd: star-matmul
        Coefficients(shape=(2, 2)) * Decision(shape=2),  # xpd: star-matmul
        Decision((2,)) * Coefficients((2, 3)),  # xpd: star-matmul
        coefficients @ x,
    )


def incompatible_product() -> cp.Expression:
    return Coefficients((2, 2)) * Decision(3)


def initialized_product() -> cp.Expression:
    coefficients = Coefficients((3, 2))
    x = Decision(2)
    x.value = [1.0, 2.0]
    return coefficients * x  # xpd: star-matmul


def unpacked_product() -> cp.Expression:
    coefficients = Coefficients((3, 2))
    x = Decision(2)
    x, _ = 2.0, 0
    return coefficients * x


def rebound_coefficient() -> cp.Expression:
    coefficients = Coefficients((2, 2))
    coefficients = Coefficients((2, 3))
    return coefficients * Decision(2)


def identity_forms() -> tuple[cp.Expression, ...]:
    x = Decision(2)
    return (
        quadratic(x, eye(2)),  # xpd: dense-identity-quad-form
        quadratic(Decision(2), dense_identity(2)),  # xpd: dense-identity-quad-form
    )


def differently_sized_identity() -> cp.Expression:
    x = Decision(3)
    return quadratic(x, eye(2))


def shadowed_eye(eye) -> cp.Expression:
    return quadratic(Decision(2), eye(2))


def shadowed_constructor() -> cp.Expression:
    from cvxpy import Parameter as Decision

    return quadratic(Decision(2), eye(2))

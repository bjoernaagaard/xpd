import cvxpy as cp


def rejected():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.sqrt(cp.square(x))))  # xpd: non-dcp-problem
    problem.solve()


def explicit_qcp():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.sqrt(cp.square(x))))
    problem.solve(qcp=True)


def unchecked():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.abs(x)))
    problem.solve()  # xpd: unchecked-solver-status
    return problem.value


def checked():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.abs(x)))
    problem.solve()
    if problem.status == cp.OPTIMAL:
        return problem.value


def mixed():
    exposure = cp.Variable(name="exposure [USD]")
    cap = cp.Parameter(name="cap [EUR]")
    return cp.Problem(cp.Minimize(exposure), [exposure <= cap])  # xpd: constraint-unit-mismatch


def converted():
    exposure = cp.Variable(name="exposure [USD]")
    cap = cp.Parameter(name="cap [EUR]")
    return cp.Problem(cp.Minimize(exposure), [exposure <= 1.1 * cap])

import cvxpy as cp


def rejected():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.sqrt(cp.square(x))))
    problem.solve()


def explicit_qcp():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.sqrt(cp.square(x))))
    problem.solve(qcp=True)


def unchecked():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.abs(x)))
    problem.solve()
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
    return cp.Problem(cp.Minimize(exposure), [exposure <= cap])


def converted():
    exposure = cp.Variable(name="exposure [USD]")
    cap = cp.Parameter(name="cap [EUR]")
    return cp.Problem(cp.Minimize(exposure), [exposure <= 1.1 * cap])

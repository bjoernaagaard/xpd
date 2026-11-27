---
title: Review numerical optimization
description: Record evidence for a CVXPY, pymoo, or skfolio problem and gate changes against project checks.
---

Run `xpd opt-review <paths> --limit 0 --format json` to list problem IDs, findings and questions. A CVXPY solve can complete without a usable solution. A pymoo front can contain infeasible candidates. A fitted portfolio can include returns from its holdout. Check the evidence appropriate to the problem.

## Measure an actual solution

In a Git project, save this as `solve.py`.

```python title="solve.py"
import cvxpy as cp


def build():
    x = cp.Variable()
    problem = cp.Problem(cp.Minimize(cp.abs(x)), [x >= 1])
    return problem


if __name__ == "__main__":
    problem = build()
    problem.solve(solver="SCS", eps_abs=1e-8, eps_rel=1e-8)
    assert problem.status == cp.OPTIMAL
    assert abs(problem.value - 1) < 1e-6
    assert problem.constraints[0].violation() < 1e-6
```

Copy the problem ID from `xpd opt-review solve.py --format json | jq -r '.read.problems[0].id'` into `adapter.py`.

```python title="adapter.py"
from solve import build


def problems():
    return {
        "<problem-id>": {
            "library": "cvxpy",
            "problem": build(),
            "solve_kwargs": {"solver": "SCS", "eps_abs": 1e-8, "eps_rel": 1e-8},
        },
    }
```

The adapter reports the solver status and the named metrics for this problem.

```console
$ git init -q
$ python solve.py
$ sed -i "s/<problem-id>/$(xpd opt-review solve.py --format json | jq -r '.read.problems[0].id')/" adapter.py
$ xpd opt-review solve.py --adapter adapter.py --format json | jq -r '.read.facts.problems[0].facts["solver-status"].value.status'
optimal
```

The example check uses tolerances for this fixture. Choose tolerances and a fallback for the real decision.

## Record and finish

1. Commit the project files and run `xpd opt-review start solve.py --check "python solve.py" --adapter adapter.py`.
2. List again with the same paths and adapter. Run `xpd opt-review plan plan.json` with the findings or checklist questions a fix addresses. Record findings you accept and checklist verdicts with reasons.
3. If the fix changes an interface, run `xpd opt-review approve <fix>`. Commit the change and run `xpd opt-review gate <fix>`.
4. Read the next stage with `xpd opt-review status`. To abandon a fix, restore the last kept commit and run `xpd opt-review drop <fix> --reason "<reason>"`.
5. Record remaining or revised answers with `xpd opt-review plan -`. Run `xpd opt-review finish`.

See the [command reference](../../commands/opt-review/) for the adapter contract and all stages.

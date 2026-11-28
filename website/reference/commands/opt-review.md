Use `xpd opt-review` to review CVXPY, pymoo, and skfolio problem definitions, solver results and validation.

Run `xpd opt-review <paths> --limit 0 --format json` to list `read.problems`, `read.facts`, `findings` and `checklist`. Problem IDs come from repository-relative paths and qualified definition sites. Comments above a definition do not change its ID. The inventory skips dynamic factories and ambiguous bindings. A static-only list has null `read.facts`. Use `--questions` to print the catalog without scanning or running an adapter.

```console
$ xpd opt-review --questions --format json | jq -c '{facts:(.facts|length),questions:(.questions|length)}'
{"facts":12,"questions":12}
```

## Recorded stages

| Command | Result |
| --- | --- |
| [`xpd opt-review start`](start/) | Record committed paths, optional adapter and baseline project checks. |
| `xpd opt-review <paths>` | List problems, findings, facts and checklist items. |
| [`xpd opt-review plan`](plan/) | Record fixes, accepted findings and reasoned checklist answers. |
| [`xpd opt-review approve`](approve/) | Approve an interface change. |
| [`xpd opt-review gate`](gate/) | Compare committed surveys and run the checks that passed at start. |
| [`xpd opt-review drop`](drop/) | Explain an abandoned fix after restoring the last kept commit. |
| [`xpd opt-review status`](status/) | Read the run and next stage. |
| [`xpd opt-review finish`](finish/) | Close the run when fixes, findings and checklist items are settled. |

## Adapters

Use `--adapter adapter.py` or pass a command that prints normalized JSON. Adapters run at the selected project root. Commit the adapter and its inputs before recording a run. The Python runner calls `problems()`, which returns one descriptor per inventory ID.

| Library | Descriptor fields |
| --- | --- |
| `cvxpy` | `problem` and optional `solve_kwargs`. |
| `skfolio` | `runs`, each with an `estimator`, `fit_args`, `fit_indices`, optional `fit_kwargs` and optional `evaluation_indices`. Index positions must share an input axis. |
| `pymoo` | `problem`, `algorithm`, `seed`, `termination`, checkpoint generations in `checkpoints`, a fixed `reference_point` for multiobjective problems and optional positive `objective_scales`. |

Any descriptor can declare `matrices` as a name-to-array mapping. To label a matrix, provide a mapping with `matrix`, `role` and `units`. Use `unmeasured` with a reason for an intentionally unmeasured problem.

Commands emit the same `{"problems":[{"id":"<id>","library":"cvxpy","facts":{}}]}` shape as `read.facts`, with one entry per inventory ID. Each measured fact includes `value` and `method`. An unavailable value is null and includes a `reason`. Use the catalog for fact names. Invalid payloads fail a list or recorded action.

See the [optimization review guide](../../guides/opt-review/) for a recorded example.

## Recorded example

Save this as `solve.py`.

```python title="solve.py"
import cvxpy as cp


def solve():
    x = cp.Variable()
    p = cp.Problem(cp.Minimize(cp.abs(x)), [x >= 1])
    p.solve(solver="SCS", eps_abs=1e-8, eps_rel=1e-8)
    return p.value
```

Commit it alongside this project check in `solution.py`.

```python title="solution.py"
from solve import solve

assert abs(solve() - 1) < 1e-6
```

```console
$ git init -q && git config user.name Example && git config user.email example@example.invalid && git config commit.gpgsign false && git add solve.py solution.py && git commit -qm 'Record the solve'
$ xpd opt-review start solve.py --check 'python solution.py' --format json | jq -r '.next.stage'
plan
$ xpd opt-review solve.py --limit 0 --format json | jq -c '[.findings[].kind]'
["unchecked-solver-status"]
$ xpd opt-review solve.py --format json | jq '{fixes:[{id:"fix-status",title:"Check solver status",findings:[.findings[].id]}]}' | xpd opt-review plan - --format json | jq -r '.next.stage'
fix
```

Update the return to run only under `if p.status == cp.OPTIMAL:` and commit the fix. The gate reads the new code and reruns the solution check.

```console
$ python -c 'from pathlib import Path; p = Path("solve.py"); p.write_text(p.read_text().replace("    return p.value", "    if p.status == cp.OPTIMAL:\n        return p.value"))' && git add solve.py && git commit -qm 'Check solver status'
$ xpd opt-review gate fix-status --format json | jq -r '.run.rounds[-1][0].outcome'
kept
$ xpd opt-review solve.py --format json | jq -c '[.findings[].kind]'
[]
```

Inspect the checklist and record your reasons before finishing. The scratch review accepts the items with a reason because it makes no allocation decision.

```console
$ xpd opt-review solve.py --format json | jq '{answered:[.checklist[]|{item:.id,verdict:"accepted",reason:"Scratch solve verified against its known optimum; no allocation decision is made."}]}' | xpd opt-review plan - --format json | jq -r '.next.stage'
plan
$ xpd opt-review finish --format json | jq -r '.next.stage'
xpd: 1 findings at the start, 0 now: 1 fixed, 0 added, 0 accepted, 0 left without a reason; lines 8 -> 9
finished
```

## Exit status

A successful list or stage exits 0. A rejected gate or blocked finish exits 1. Invalid input and adapter errors exit 2.

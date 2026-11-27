# Review numerical optimization and portfolio solves

Use `xpd opt-review` to audit the objective, constraints, solver evidence, and portfolio evaluation of a numerical optimization problem.

## Asks

- audit numerical optimization and portfolio solves
- check CVXPY DCP and solver acceptance
- review skfolio backtest leakage
- assess pymoo Pareto front convergence

## Code

```python
import cvxpy as cp

x = cp.Variable()
problem = cp.Problem(cp.Minimize(cp.abs(x)), [x >= 1])
```

## Commands

1. List problems with `xpd opt-review <paths> --limit 0 --format json`. Add a Python or command adapter to record solver evidence.
2. Commit inputs and run `xpd opt-review start <paths> --check "<solution check>"`. Use checks that evaluate the actual solution at meaningful tolerances.
3. Run `xpd opt-review plan plan.json` with finding IDs, design question IDs, and reasoned answers. Use `xpd opt-review approve <fix>` for an interface change.
4. Commit a fix and run `xpd opt-review gate <fix>`. Use `xpd opt-review status` to resume or `xpd opt-review drop <fix> --reason "<reason>"` after restoring the last kept commit.
5. Record any remaining answers with `xpd opt-review plan -` and close with `xpd opt-review finish`.

Use `xpd find 'walk forward portfolio cross validation' --package skfolio`, `xpd api skfolio.model_selection.WalkForward`, `xpd api pymoo.optimize.minimize`, and `xpd api cvxpy/non-dcp-problem` to check supported calls and patterns.

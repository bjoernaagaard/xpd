---
name: xpd-opt-review
description: Review numerical optimization and portfolio formulations, solver diagnostics, Pareto convergence, and holdout hygiene in CVXPY, pymoo, and skfolio with a recorded xpd opt-review ledger.
---

# Review numerical optimization with xpd

Use `xpd opt-review` to verify CVXPY, pymoo, and skfolio problem formulations, inspect real solver statuses and residuals, check portfolio leakage, and gate every change against solution tolerances.

| Group | Finding kinds |
| --- | --- |
| Formulation | `non-dcp-problem`, `constraint-unit-mismatch` |
| Solving | `unchecked-solver-status` |
| Portfolio validation | `fit-on-holdout` |
| Pack rules | `pack-rule` |

Run `xpd opt-review --questions` for the 12-fact and 12-question catalog and `xpd api xpd/recipe/opt-review` for the playbook.

## Adapter

Pass `--adapter adapter.py` where `problems()` returns one descriptor per problem ID (`cvxpy` with `problem` and `solve_kwargs`, `skfolio` with `runs` and split indices, or `pymoo` with `problem`, `algorithm`, `seed`, `termination`, `checkpoints`, and `reference_point`). Measure the exact formulation and fit inputs, keep the final holdout out of development decisions, and record solver status, residual units, and any unavailable gaps or duals.

## Recorded run

1. **Survey**: run `xpd opt-review <paths> --limit 0 --format json` to list problem IDs, findings, and checklist IDs. Trace objectives, units, constraints, and API rules with `xpd find` and `xpd api`.
2. **Start and list**: commit the code, adapter, and inputs, then run `xpd opt-review start <paths> --check "<solution-check>" --adapter adapter.py` and `xpd opt-review <paths> --adapter adapter.py --format json`. Write the project check so it verifies real solution values, solver status, or Pareto front properties against project tolerances.
3. **Plan**: run `xpd opt-review plan plan.json` with ordered `fixes` (finding IDs or `"goal": "design"` with `questions`), `accepted` findings, and `answered` verdicts (`pass`, `na`, or `accepted`) with evidence.
4. **Approve and gate**: run `xpd opt-review approve <fix>` for approved interface changes, commit one fix, and run `xpd opt-review gate <fix>`. A `design` fix must edit a file owning a cited question.
5. **Drop, status, and finish**: restore the last kept commit before `xpd opt-review drop <fix> --reason "<reason>"`, resume with `xpd opt-review status`, update revised answers with `xpd opt-review plan -`, and close with `xpd opt-review finish`.

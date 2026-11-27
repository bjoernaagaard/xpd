---
name: xpd-opt-review
description: Review the formulation, solver evidence, and validation of numerical optimization in CVXPY, pymoo, and skfolio. Use for DCP, solver status, duality gap, binding constraints, Pareto convergence, and portfolio leakage.
---

# Review numerical optimization

1. Run `xpd opt-review <paths> --limit 0 --format json` to list problems, findings, and checklist IDs. Run `xpd opt-review --questions` for the evidence catalog and `xpd api xpd/recipe/opt-review` for the playbook.
2. Trace objectives, units, constraints, and inputs. Use `xpd find` and `xpd api` for library calls and rules. Assess skfolio folds and costs on development data. Keep the final holdout out of development decisions.
3. Commit the selected code, adapter, and inputs. Run `xpd opt-review start <paths> --check "<solution check>" --adapter adapter.py`. Use the list with the same adapter to inspect the solve. The project check should evaluate actual solution values, statuses, or front properties against project tolerances.
4. Run `xpd opt-review plan plan.json`. Cite stable finding IDs for mechanical fixes and question IDs for design fixes. Record evidence and reasons for accepted findings and verdicts. Use `xpd opt-review approve <fix>` before gating an interface change.
5. Commit the fix and run `xpd opt-review gate <fix>`. A design fix must change a file owning a cited question. Use `xpd opt-review status` to resume. Restore the last kept commit before `xpd opt-review drop <fix> --reason "<reason>"`.
6. Record revised answers with `xpd opt-review plan -` if later changes affect their evidence. Answer each current question, including a reason for every `na`. Run `xpd opt-review finish` to close the review.

Adapter imports and commands run as project code. Python adapters return one descriptor per problem ID from `problems()`. Measure the exact formulation and fit inputs. Record the reported solver status, residual definition and units, and explain unavailable gaps or duals. For a front trace, keep the reference point, scales, checkpoints and seed explicit.

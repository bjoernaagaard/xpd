---
name: xpd-model-review
description: Review how a model is validated, compared with a naive baseline, selected and calibrated. Use for cross-validation, test-set tuning, boosting early stopping, scoring and loss, held-out probabilities or conformal interval coverage.
---

# Review a model

1. Run `xpd model-review --questions` for the catalog. Use `xpd model-review <paths> --limit 0 --format json` to find named models, reportable code patterns and each model's checklist. Use `xpd api xpd/recipe/model-review` for the review sequence.
2. Identify the deployment population, available inputs and time horizon. Keep any final holdout out of the fold adapter. Use the same outer validation rows and scorer for the model and baseline. Record group or time metadata when it defines the boundary.
3. Commit the model files, adapter and its inputs. Run `xpd model-review start <paths> --check "git diff --check" --adapter adapter.py`. Repeat the paths and adapter on the bare list to read the facts. Inspect every fold, including conflicts, held-out support and the worst-performing cases.
4. Use `xpd check` and `xpd rules` for rule evidence, `xpd find` and `xpd api` for APIs, `xpd graph` for feature paths and `xpd profile` for data evidence. For compatible pipeline contracts, call `discovery_gate` and `gates` over `xpd mcp` and cite their verdicts at this commit. Use `xpd refit model/contract.json` for saved-model agreement.
5. Run `xpd model-review plan plan.json` to record fixes, accepted findings and checklist answers with reasons. For a design fix cite question IDs and edit one of their owning files. Get approval for an interface change before `xpd model-review approve <fix>`.
6. Commit a fix and run `xpd model-review gate <fix>`. Restore an abandoned edit before `xpd model-review drop <fix> --reason "<reason>"`. Use `xpd model-review status` to find the next stage.
7. Answer remaining current questions with `xpd model-review plan -`. Use `na` only with an applicability reason. Use `xpd model-review finish` to close the ledger. The owner decides whether to deploy.

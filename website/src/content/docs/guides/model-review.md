---
title: Review model validation
description: Measure development folds and record why a model's validation, baseline and calibration suit its intended use.
---

Use `xpd model-review` before trusting a reported model score. Decide which rows represent the deployment population, which data will be available at prediction time, and which cases would hurt first if the model fails.

1. Run `xpd model-review --questions` and `xpd model-review <paths> --limit 0 --format json`. Read the owner file, findings and questions for each named model.
2. Write a [fold adapter](../../commands/model-review/#adapter) with the real split and scorer, a usable naive estimator and any applicable group or time metadata. Reserve the final test outside the adapter. Use identical validation rows for the model and baseline.
3. Commit the adapter and input code. Run `xpd model-review start <paths> --adapter adapter.py --check "git diff --check"`. Repeat the bare list with the same paths and adapter. Compare the individual fold scores, paired baseline lift and train gap. Look at reliability-bin counts or interval coverage and widths when applicable.
4. Run `xpd model-review plan plan.json` with ordered fixes and reasons for decisions. Use `xpd model-review approve <fix>` for an agreed interface change. Commit each fix and run `xpd model-review gate <fix>`. To abandon a fix, restore the last kept commit and run `xpd model-review drop <fix> --reason "<reason>"`.
5. Use `xpd model-review status` to see the next stage. Record remaining answers with `xpd model-review plan -`, then run `xpd model-review finish`. A finished review records what was addressed. The owner decides whether to deploy.

Use `xpd profile` for raw data, `xpd graph` to trace inputs, and `xpd check`, `xpd rules`, `xpd find` and `xpd api` to investigate code and APIs. For a compatible saved pipeline, run [the MCP gates](../gates/) at the reviewed commit and cite the named verdicts in the pipeline-evidence answer. Use `xpd refit model/contract.json` when only saved-model and recorded-score agreement apply. Explain why the pipeline contract is inapplicable for other validation designs.

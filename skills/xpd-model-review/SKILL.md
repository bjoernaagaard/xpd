---
name: xpd-model-review
description: Review model validation design, baseline comparisons, selection, probability calibration, and conformal interval coverage in a recorded xpd model-review ledger.
---

# Review model validation with xpd

Use `xpd model-review` before trusting a reported model score. It inventories named scikit-learn models, MAPIE wrappers, LightGBM, and XGBoost estimators, measures development folds against a naive baseline, and tracks every validation judgment in a recorded ledger.

| Group | Finding kinds |
| --- | --- |
| Validation and selection | `split-mismatch`, `test-set-tuning`, `validation-leakage`, `validation-reproducibility`, `boosting-validation` |
| Comparisons and metrics | `baseline-not-observed`, `metric-loss-mismatch`, `metric-response` |
| Calibration | `conformal-leakage`, `conformal-configuration` |

Run `xpd model-review --questions` for the full evidence catalog and `xpd api xpd/recipe/model-review` for the playbook.

## Adapter and contracts

- **Development fold adapter (`--adapter adapter.py`)**: define `models()` returning descriptors with `file`, `binding`, `X`, `y`, `estimator`, `scorer`, `splitter`, disjoint `folds`, optional `baseline`, `groups`, `times`, `positive_label`, `fit_fold`, and `conformalize`. Keep any final holdout outside the adapter, and score the model and baseline on identical validation rows.
- **Saved pipeline contracts**: for a compatible saved scikit-learn Pipeline, run `discovery_gate` and `gates` over `xpd mcp` at the reviewed commit and cite their verdicts, or run `xpd refit model/contract.json` for saved-model agreement.

## Recorded run

1. **Survey**: run `xpd model-review <paths> --limit 0 --format json` to read named models, findings, and checklist IDs. Inspect inputs and lineage with `xpd profile`, `xpd graph`, `xpd check`, `xpd rules`, `xpd find`, and `xpd api`.
2. **Start and list**: commit the model files, adapter, and inputs, then run `xpd model-review start <paths> --check "<project-check>" --adapter adapter.py` and `xpd model-review <paths> --adapter adapter.py --format json`. Inspect every fold, train-to-validation gap, worst-performing cases, reliability bins, and conformal interval coverage.
3. **Plan**: run `xpd model-review plan plan.json` with ordered `fixes` (finding IDs or `"goal": "design"` with `questions`), `accepted` findings, and `answered` verdicts (`pass`, `na`, or `accepted`) with reasons.
4. **Approve and gate**: run `xpd model-review approve <fix>` for approved interface changes, commit one fix, and run `xpd model-review gate <fix>`.
5. **Drop, status, and finish**: restore an abandoned edit before `xpd model-review drop <fix> --reason "<reason>"`, resume with `xpd model-review status`, record any remaining answers with `xpd model-review plan -`, and close with `xpd model-review finish`. The owner decides whether to deploy.

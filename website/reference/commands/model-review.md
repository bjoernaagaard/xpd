Use `xpd model-review` to review validation, selection, baseline comparisons and calibration in Python. The list inventories named scikit-learn models and pipelines, MAPIE wrappers, and LightGBM and XGBoost estimators. It forwards selected pack reports and adds local advisories about split design, baseline comparison and quantile scoring.

## List and catalog

```console
$ xpd model-review --questions --format json | jq -c '[.questions[].id]'
["validation-scheme","baseline","metric-and-loss","selection","probability-calibration","interval-coverage","first-failure","pipeline-evidence"]
```

The catalog includes 15 names from the pipeline proof. Listing the catalog runs no project code.

Run `xpd model-review <paths> --limit 0 --format json` to list findings, inventory and checklist. Add `--adapter adapter.py` for runtime measurements. Repeat the recorded paths and adapter when listing during a run. `read.models` identifies each model by `file` and qualified `binding`. The stable model ID prefixes its checklist IDs. Resolved findings include a `model` ID. Unresolved findings still appear. With no adapter, `read.facts` is null. Adapter errors appear under `read.facts.error` and leave the static list available. Recorded stages require valid measurements when an adapter is configured.

## Adapter

A Python adapter defines `models()` and returns an iterable of mappings.

| Field | Meaning |
| --- | --- |
| `file`, `binding` | A selected Git-root-relative Python file and a qualified binding such as `train.model`. Give factories and adapter-only models distinct bindings. |
| `X`, `y`, `estimator` | Row-aligned development data and an unfitted estimator. Exclude any final test set. |
| `scorer`, `splitter` | A scikit-learn scorer name and the name of the splitter used to materialize folds. Scores use the scorer's higher-is-better convention. |
| `folds` | Mappings with a unique string `id`, integer positional `fit` and `validation` indices and optional `early_stopping` and `calibration` indices. Roles must be disjoint within each fold. |
| `baseline` | Optional unfitted estimator. Defaults to a prior classifier or mean regressor. Both estimators use the exact same fit and validation rows. |
| `groups`, `times` | Optional row-aligned group IDs and UTC timestamps. The report shows group intersections, time boundaries and conflicts per fold. |
| `positive_label` | Required to measure binary Brier loss and five reliability bins using held-out probability events. |
| `fit_fold` | Optional callback taking the fresh clone, fit features and labels, then early-stopping features and labels or two `None` values. Return the fitted estimator. |
| `conformalize`, `confidence_levels` | Optional callback taking the fitted estimator and calibration features and labels. Return a conformalizer with `predict_interval` producing an array shaped `(rows, 2, levels)` as its second result. Declare the levels in that order. |

A shell adapter prints one JSON document with the same `models` array as the Python runner. To see the schema with concrete values, run a Python adapter and read `read.facts`. Every model includes `file`, `binding`, `scorer`, `splitter`, `baseline`, `folds`, `spread`, `probability_calibration` and `interval_coverage`. The measurements include paired scores, the train-to-validation gap, population standard deviation and range, reliability-bin support, and pooled and per-fold interval coverage. Repeated validation rows count as separate prediction events. Unavailable measurements include a reason.

Recorded Python adapters must be committed in the repository. Recorded adapters run from the root of the selected commit on both sides of a comparison. Project Python logs go to stderr.

## Recorded stages

| Command | Result |
| --- | --- |
| [`xpd model-review start`](start/) | Record the clean tracked baseline, paths, checks and optional adapter. |
| `xpd model-review <paths>` | List current models, findings, facts and questions. |
| [`xpd model-review plan`](plan/) | Record fixes, acceptance reasons and checklist verdicts. |
| [`xpd model-review gate`](gate/) | Evaluate one committed fix against the last kept commit. |
| [`xpd model-review finish`](finish/) | Compare with the starting commit and close the run. |
| [`xpd model-review status`](status/) | Show the run and next stage. |
| [`xpd model-review approve`](approve/) | Approve a fix that changes an interface. |
| [`xpd model-review drop`](drop/) | Record why a fix was abandoned. |

## Recorded example

A quantile model predicts at 0.9. The score mistakenly uses a pinball loss at 0.1.

```python title="bad.py"
from sklearn.ensemble import GradientBoostingRegressor
from sklearn.metrics import mean_pinball_loss


def score_quantile(x_fit, y_fit, x_validation, y_validation):
    model = GradientBoostingRegressor(loss="quantile", alpha=0.9, random_state=7)
    model.fit(x_fit, y_fit)
    return mean_pinball_loss(y_validation, model.predict(x_validation), alpha=0.1)
```

The example initializes a scratch Git repository. It prints the next stage or fix outcome after each successful command and the refusal message when a fix or an answer is open.

```console
$ git init -q && git -c user.name=Example -c user.email=example@example.com add bad.py && git -c user.name=Example -c user.email=example@example.com commit -qm initial
$ xpd model-review start bad.py --check "git diff --check" --format json | jq -r .next.stage
plan
$ xpd model-review bad.py --limit 0 --format json | jq -c '[.findings[].kind]'
["metric-loss-mismatch"]
$ xpd model-review bad.py --limit 0 --format json | jq '{fixes:[{id:"pinball",title:"Score the intended quantile",findings:[.findings[] | select(.kind == "metric-loss-mismatch") | .id]}]}' > plan.json
$ xpd model-review plan plan.json --format json | jq -r .next.stage
fix
$ xpd model-review finish --format json 2>&1 >/dev/null
xpd: the fix pinball is still open
$ python -c 'from pathlib import Path; p = Path("bad.py"); p.write_text(p.read_text().replace("alpha=0.1", "alpha=0.9"))' && git -c user.name=Example -c user.email=example@example.com commit -qam 'Score the intended quantile'
$ xpd model-review gate pinball --format json | jq -r '.run.rounds[-1][0].outcome'
kept
$ xpd model-review finish --format json 2>&1 >/dev/null | cut -d: -f1-2
xpd: these checklist items are unanswered
$ xpd model-review bad.py --format json | jq '{answered:[.checklist[] | {item:.id,verdict:"accepted",reason:"Scratch example with no deployment; validation design remains unverified."}]}' | xpd model-review plan - --format json | jq -r .next.stage
plan
$ xpd model-review finish --format json 2>/dev/null | jq -r .next.stage
finished
```

See the [model review guide](../../guides/model-review/) for the review sequence.

## Exit status

| Status | Meaning |
| --- | --- |
| 0 | A successful stage or list. JSON stage output contains `run` and `next`. |
| 1 | A rejected gate, open fix, unanswered question or unexplained new finding. |
| 2 | An invocation or adapter error. |

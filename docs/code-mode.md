# Code mode

!!! note "First draft"

    This page is a first draft. Discovery now starts the workflow, and the agent records its
    choice before the checks. It describes one way for an agent to take a data file to a
    saved scikit-learn model through `xpd mcp --code-mode`, with checks that xpd runs itself.
    The tool names, the contract and the gates may change. The last section lists what is
    open.

With [`--code-mode`](mcp.md#settings), the server shows three tools in place of its own:
`search`, `get_schema` and `execute`. This is FastMCP's
[code mode](https://gofastmcp.com/docs/servers/transforms/code-mode). The agent sends a short
Python script to `execute`, the script calls xpd's tools one after another, and only what the
script returns goes back to the agent.

```mermaid
sequenceDiagram
    participant Agent
    participant Sandbox as execute, in the sandbox
    participant Server as xpd's tools
    participant Python as the project's Python
    Agent->>Sandbox: one script
    Sandbox->>Server: call_tool("profile"), then "check", then "graph"
    Sandbox->>Server: call_tool("equiv") twice
    Server->>Python: train.py twice, then train.py and predict.py
    Sandbox->>Server: call_tool("contract"), then call_tool("refit")
    Server->>Python: refit from raw training rows
    Sandbox->>Server: call_tool("gates") with every result
    Sandbox-->>Agent: passed, and one line per gate
```

## What the sandbox can do

The design follows what FastMCP documents for code mode, and it uses the default settings.

| Fact | What it means here |
| --- | --- |
| `call_tool(name, params)` is the only function in the sandbox | The script cannot import scikit-learn or read a file. The model code is in project files, and the script runs xpd's tools on them |
| `call_tool` returns the tool's structured result | The script hands one tool's result to the next tool as it is |
| A tool runs in the server, with the same checks as a direct call | A path outside the project root is still refused |
| The sandbox stops a script after 30 seconds and 100 MB, and the time a tool takes does not count | A long training run does not stop the script |
| One `execute` call makes at most 50 tool calls | The pipeline script below makes up to 21 |
| Prompts and resources are not tools | The agent reads the `build_pipeline` prompt through its client, before it calls `execute` |

## What worked before this draft

Code mode could already chain `profile`, `check`, `graph` and `equiv` in one `execute` call.
Nothing said what a finished model run looks like, and nothing judged the results together.
An agent had to decide for itself whether a run was good.

## What the draft adds

| Addition | Kind | What it does |
| --- | --- | --- |
| `discovery_gate` | Tool | Reads direct imported calls in the chosen modules and compares their names and literal keywords with `api` |
| The stages of a run | Typed models in `python/xpd/mcp/models.py` | Name what each stage hands to the next |
| `contract` | Tool | Reads the JSON file that the training script saves beside its model |
| `refit` | Tool and command | Fits a clone of the saved Pipeline on raw training rows and compares outputs and scores |
| `gates` | Tool | Takes the results of the other tools and returns one verdict, with one line per requirement |
| `build_pipeline` | Prompt | Tells the agent which three files to write, and holds the script for `execute` |
| `samples/pipeline/` | Sample project | A run that passes every gate, and a leaky script that stops at the held out selection rule |
| `xpd equiv` in two processes | Command | Runs each script in its own Python process, so the prediction script gets no module and no path from the training script |

The `contract` and `gates` tools run no Python and load no model. The server needs no new
package. The project's Python needs pandas and scikit-learn.

## The stages

A run has nine stages when it starts with discovery. Each one hands a typed result to the next.

| Stage | What it hands on | Model | Where it comes from |
| --- | --- | --- | --- |
| Discovery | The task, inspected candidates and the choice with its reason | `DiscoveryQuestion`, `DiscoveryEntry`, `DiscoveryChoice`, `Discovery` | `find`, `api` and the agent |
| Raw data | The file, the target column and the row count | `RawData` | The contract |
| Profile | Each column's dtype, nulls and distinct values | `Profile` | The `profile` tool |
| Split | The seed, the share held out and the rows on each side | `Split` | The contract |
| Preprocessing | The raw columns the model takes, and each step of the one Pipeline | `InputColumn`, `FittedStep` | The contract |
| Model | The last step of the Pipeline | `FittedStep` | The contract |
| Validation | One score per fold, and one score for the held out rows | `Validation` | The contract |
| Proof | What `check`, `graph`, `equiv` and `refit` found | `Proof` | Those four tools |
| Persisted model | The model file, the class of the saved object, the versions that wrote it and the code files it imports | `SavedModel` | The contract |

The contract is one JSON file with those parts. The JSON below shows the model fields;
the sample also records the discovery object shown in the discovery section. The training script writes it, and it names
every file that a script needs to serve the model: the data, the model and the code under
`model.code`. joblib saves the function of a `FunctionTransformer` by its module and name, so
the sample's model loads only where `features.py` can be imported. This is the contract of the
sample run.

```json
{
  "data": {"file": "penguins.csv", "target": "species", "rows": 344},
  "split": {"seed": 0, "test_size": 0.25, "stratified": true, "train_rows": 258, "test_rows": 86},
  "inputs": [
    {"column": "island", "dtype": "str"},
    {"column": "sex", "dtype": "str"},
    {"column": "bill_length_mm", "dtype": "float64"},
    {"column": "bill_depth_mm", "dtype": "float64"},
    {"column": "flipper_length_mm", "dtype": "float64"},
    {"column": "body_mass_g", "dtype": "float64"}
  ],
  "steps": [
    {"name": "features", "estimator": "FunctionTransformer"},
    {"name": "columns", "estimator": "ColumnTransformer"},
    {"name": "classifier", "estimator": "LogisticRegression"}
  ],
  "validation": {
    "splitter": {
      "class": "StratifiedKFold",
      "args": {
        "n_splits": 5,
        "shuffle": true,
        "random_state": 0
      }
    },
    "scoring": "accuracy",
    "folds": 5,
    "cv_scores": [
      0.9807692307692307,
      0.9807692307692307,
      1.0,
      1.0,
      0.9803921568627451
    ],
    "test_score": 0.9883720930232558
  },
  "model": {
    "file": "model/model.joblib",
    "type": "sklearn.pipeline.Pipeline",
    "versions": {
      "python": "3.15.2",
      "scikit-learn": "1.9.1",
      "pandas": "3.1.1",
      "numpy": "2.5.5",
      "joblib": "1.6.0"
    },
    "code": ["features.py"]
  }
}
```

## The gates

In this draft, a run is done when it passes the thirteen gates below and, when discovery is supplied, the two discovery gates. `gates` returns each one
with `passed` and a `detail`. The last column names the tools whose results decide the gate.

A gate shows only what its column says. Refit checks outputs and scores computed from the raw file. It does not prove that a run
has no leak. [What is open](#what-is-open) lists the remaining limits.

| Gate | Stage | The run passes when | Decided by |
| --- | --- | --- | --- |
| `indexed-api-calls` | Discovery | `discovery_gate` confirms direct imported callable names and literal keyword names in the chosen modules | `api`, source files |
| `discovery-is-recorded` | Discovery | The contract and proof have the same discovery record | `contract`, supplied proof |
| `data-is-profiled` | Raw data | The contract names the profiled file with its row count, and the target has no nulls | `profile`, `contract` |
| `split-covers-every-row` | Split | The training rows and the held out rows add up to the rows of the file | `profile`, `contract` |
| `split-is-seeded` | Split | `check` reports no draw without a seed in scikit-learn, scikit-lego, MAPIE, Category Encoders, arch or pymoo code, by the rules in the table below | `check`, `contract` |
| `inputs-are-file-columns` | Preprocessing | Each input of the contract has the name and dtype of a column of the file, and none is the target. The values are not compared | `profile`, `contract` |
| `dataflow-connects` | Preprocessing | `graph` reports no error or warning, and each feature step reads only input columns and columns of earlier steps | `graph`, `contract` |
| `one-pipeline` | Model | The contract names the saved object as `sklearn.pipeline.Pipeline` and lists its steps | `contract` |
| `check-reports-no-error` | Proof | `check` reports no error of any pack. The table below names the errors that are leaks | `check` |
| `cross-validated` | Validation | The contract has one score for each of two or more folds. The refit checks their scores on training rows | `contract` |
| `reproducible` | Proof | Two training runs in separate Python processes leave equal outputs | `equiv` |
| `refit-matches-model` | Proof | A clone fitted on raw training rows gives equal held out outputs | `refit` |
| `refit-matches-scores` | Validation | The recorded held out score, fold scores and row and fold counts equal those computed by refit | `refit`, `contract` |
| `saved-model-predicts` | Persisted model | `equiv` of the training script with the prediction script is equivalent, so a new Python process that loads the model from disk leaves the `result` of the fitted one | `equiv` |
| `versions-recorded` | Persisted model | The contract records Python, scikit-learn and pandas, and its pandas is the one `profile` ran with | `profile`, `contract` |

`check-reports-no-error` counts every error of every pack, and not a list of rules.
`split-is-seeded` reads a list of rules, and the server refuses to start when the binary no
longer has one of them. Its rules are from the six packs in the table below. A draw from another
library, such as `np.random.default_rng()` without a seed, is in no list, but can fail `refit-matches-model` when it changes the fitted outputs.

| Pack | Rules that fail `split-is-seeded` | Errors about a leak that fail `check-reports-no-error` |
| --- | --- | --- |
| sklearn | `missing-random-state`, `random-state-without-shuffle` | `fit-before-split`, `fit-on-test`, `tune-on-test` |
| sklego | `unseeded-random-estimator` | None. Its errors are wrong numbers and code that raises |
| mapie | `unseeded-resampling` | `calibrate-on-training-rows`, `coverage-on-seen-rows` |
| category_encoders | `encoder-noise-without-seed` | `encode-before-split`, `transform-with-test-target` |
| arch | `arch-unseeded-draws` | None. Its errors are removed names, unknown options and code that raises |
| pymoo | `unseeded-run`, `numpy-seed-ignored` | None. Its errors are removed names, ignored arguments, wrong results and code that raises |

## A run from raw rows to a saved model

`samples/pipeline/` is a small project. Its data is the
[Palmer penguins](https://github.com/allisonhorst/palmerpenguins) table, which is under the
CC0 license: 344 rows, with nulls in five columns.

```text
samples/pipeline/
├── discovery.json  the inspected planning record copied into the training contract
├── grouped.py      one fitted model per group, for 24 groups of made up rows
├── bootstrap.py    arch and MAPIE block resampling, and forward validation
├── figure.py       a new Plotly scatter figure
├── invalid_api.py  made up API names that the discovery gate refuses
├── penguins.csv     the raw rows
├── features.py      two feature steps, as step code that graph reads
├── train.py         split, one Pipeline, cross-validation, the saved model and its contract
├── predict.py       loads the saved model from the contract and gives the class probabilities of the held out rows
├── leaky.py         train.py with one leak
└── model/           what train.py writes. Git ignores it
    ├── model.joblib
    └── contract.json
```

The client below starts the server with code mode on and sends one script. Run it in the
repository, in an environment that has xpd, pandas and scikit-learn.

```python
import asyncio
import json

from fastmcp import Client
from fastmcp.client.transports import StdioTransport

STAGES = """
DATA = "penguins.csv"
FEATURES = "features.py"
TRAIN = "train.py"
PREDICT = "predict.py"
CONTRACT = "model/contract.json"
TASK = "classify penguin species with a logistic regression pipeline"
CHOICE = "sklearn.linear_model.LogisticRegression"
REASON = "Class probabilities for the species target."
SYMBOLS = [CHOICE]
found = await call_tool("find", {"task": TASK})
inspected = {
    kind: [await call_tool("api", {"name": item["name"]}) for item in found[kind]]
    for kind in ("symbols", "rules", "recipes")
}
discovery = {
    "question": {"task": TASK}, "candidates": inspected,
    "choice": {"name": CHOICE, "reason": REASON, "symbols": SYMBOLS},
}
api_proof = await call_tool("discovery_gate", {
    "paths": [TRAIN, PREDICT, FEATURES], "discovery": discovery,
})
if not api_proof["gate"]["passed"]:
    return api_proof

profile = await call_tool("profile", {"file": DATA})
lint = await call_tool("check", {"paths": [TRAIN, PREDICT, FEATURES]})
if any(issue["severity"] == "error" for issue in lint["issues"]):
    return {"passed": False, "lint": lint, "discovery": api_proof}
dataflow = await call_tool("graph", {"paths": [FEATURES]})
reproducible = await call_tool("equiv", {"old": TRAIN, "new": TRAIN})
served = await call_tool("equiv", {"old": TRAIN, "new": PREDICT})
contract = await call_tool("contract", {"file": CONTRACT})
refitted = await call_tool("refit", {"file": CONTRACT})
proof = {
    "lint": lint, "dataflow": dataflow, "refit": refitted,
    "reproducible": reproducible,
    "served": served, "discovery": api_proof,
}
report = await call_tool("gates", {"profile": profile, "contract": contract, "proof": proof})
return {
    "passed": report["passed"],
    "gates": [
        {"name": gate["name"], "passed": gate["passed"], "detail": gate["detail"]}
        for gate in report["gates"]
    ],
    "refit": refitted,
    "validation": contract["validation"],
    "model": contract["model"],
}
"""


async def main() -> None:
    server = StdioTransport("xpd", ["mcp", "--code-mode", "--root", "samples/pipeline"])
    async with Client(server) as client:
        result = await client.call_tool("execute", {"code": STAGES})
        print(json.dumps(result.structured_content, indent=2))


asyncio.run(main())
```

The script is the one that the `build_pipeline` prompt returns for `data` set to
`penguins.csv` and `target` set to `species`. The first `equiv` call runs `train.py` twice. The second runs `train.py` and
`predict.py`, each in a new Python process. The training script saves the model and
contract, and the prediction script imports `features.py` and loads the saved model.
Then `refit` loads that Pipeline, clones it and fits it on the raw training rows.
It computes fold scores with the contract's splitter and held out scores with its scorer.
Save the full scores without rounding. The JSON contract below records the splitter too.

Refit uses the same comparisons as `equiv`. Arrays and scalar scores must be exactly equal
under `numpy.testing.assert_equal`. DataFrames and Series use pandas testing defaults.
For floating pandas values the default relative tolerance is 0.00001 and the absolute
tolerance is 0.00000001. The comparison uses probabilities when available, then decision
values, then predictions. Equality concerns those held out outputs, not every fitted
attribute or every possible input.

Unseeded NumPy noise added to the raw input values fails `refit-matches-model`. Refit does
not replace `reproducible`. A training script that draws
`C=float(np.exp(np.random.default_rng().normal()))` for LogisticRegression passed refit
with a probability gap of 0.0, equal scores and equal fold scores. Its two training runs
failed `equiv`. A clone keeps the saved value of `C`, so it does not draw that choice again.
The `reproducible` gate and its `equiv` call remain. Training runs three times, and xpd
fits its own clone once.

The sample report follows.

```json
{
  "passed": true,
  "gates": [
    {
      "name": "data-is-profiled",
      "passed": true,
      "detail": "penguins.csv has 344 rows, and the target species has no nulls."
    },
    {
      "name": "split-covers-every-row",
      "passed": true,
      "detail": "The contract counts 258 training rows and 86 held out rows."
    },
    {
      "name": "split-is-seeded",
      "passed": true,
      "detail": "The contract records the seed 0, and check reports no unseeded scikit-learn, scikit-lego, MAPIE, Category Encoders, arch or pymoo draw."
    },
    {
      "name": "inputs-are-file-columns",
      "passed": true,
      "detail": "The 6 inputs of the contract have the names and dtypes of columns of penguins.csv."
    },
    {
      "name": "dataflow-connects",
      "passed": true,
      "detail": "2 feature steps read only input columns and columns of earlier steps."
    },
    {
      "name": "one-pipeline",
      "passed": true,
      "detail": "The contract lists one Pipeline of FunctionTransformer then ColumnTransformer then LogisticRegression."
    },
    {
      "name": "check-reports-no-error",
      "passed": true,
      "detail": "check reports no error."
    },
    {
      "name": "cross-validated",
      "passed": true,
      "detail": "The contract records accuracy 0.9803921568627451 to 1.0 on 5 folds, and 0.9883720930232558 as the held out score."
    },
    {
      "name": "reproducible",
      "passed": true,
      "detail": "Two runs of the training script, each in its own Python process, leave an equal `result`."
    },
    {
      "name": "refit-matches-model",
      "passed": true,
      "detail": "The raw training refit matches the saved model on predict_proba."
    },
    {
      "name": "refit-matches-scores",
      "passed": true,
      "detail": "The contract scores and row and fold counts match the raw training refit."
    },
    {
      "name": "saved-model-predicts",
      "passed": true,
      "detail": "The prediction script, in its own Python process, leaves the `result` that the training script left."
    },
    {
      "name": "versions-recorded",
      "passed": true,
      "detail": "The contract records python 3.15.2, scikit-learn 1.9.1, pandas 3.1.1, numpy 2.5.5, joblib 1.6.0."
    },
    {
      "name": "indexed-api-calls",
      "passed": true,
      "detail": "Checked 1 direct imported calls for indexed callable names and literal keyword names in api's parameter lists. No code was run."
    },
    {
      "name": "discovery-is-recorded",
      "passed": true,
      "detail": "The contract records the discovery question, candidates and reason of the proof."
    }
  ],
  "refit": {
    "rows": 344,
    "train_rows": 258,
    "test_rows": 86,
    "output": "predict_proba",
    "largest_gap": 0.0,
    "model_matches": true,
    "contract_test_score": 0.9883720930232558,
    "refit_test_score": 0.9883720930232558,
    "contract_cv_scores": [
      0.9807692307692307,
      0.9807692307692307,
      1.0,
      1.0,
      0.9803921568627451
    ],
    "refit_cv_scores": [
      0.9807692307692307,
      0.9807692307692307,
      1.0,
      1.0,
      0.9803921568627451
    ],
    "test_score_matches": true,
    "cv_scores_match": true,
    "counts_match": true,
    "passed": true
  },
  "validation": {
    "splitter": {
      "class": "StratifiedKFold",
      "args": {
        "n_splits": 5,
        "shuffle": true,
        "random_state": 0
      }
    },
    "scoring": "accuracy",
    "folds": 5,
    "cv_scores": [
      0.9807692307692307,
      0.9807692307692307,
      1.0,
      1.0,
      0.9803921568627451
    ],
    "test_score": 0.9883720930232558
  },
  "model": {
    "file": "model/model.joblib",
    "type": "sklearn.pipeline.Pipeline",
    "versions": {
      "python": "3.15.2",
      "scikit-learn": "1.9.1",
      "pandas": "3.1.1",
      "numpy": "2.5.5",
      "joblib": "1.6.0"
    },
    "code": [
      "features.py"
    ]
  }
}
```

`predict.py` then predicts in a process of its own. Run in `samples/pipeline`, it gives the
probability of each species for the 86 held out rows, and the most likely species is right for
85 of them. These are the first three rows.

```text
     Adelie  Chinstrap  Gentoo
298  0.0461     0.9531  0.0008
157  0.0002     0.0045  0.9954
137  0.9884     0.0113  0.0003
```

The model needs `features.py` where it loads. With `model/`, `penguins.csv` and `predict.py`
copied to an empty folder, `predict.py` stops before it loads the model.

```text
FileNotFoundError: The model imports ['features.py'], and no such file is here.
```

## A leaky script stops before execution

`leaky.py` tries four strengths of regularization and keeps the one with the best score
on held out rows. The current prompt script stops at `check` when it sees
`sklearn/tune-on-test`, before the `equiv` call can run that code. To try it, set
`TRAIN = "leaky.py"` in the pipeline script above. This shows an error that the rule knows.
The same loop with `X_hold` fails too when that name is a held out target of an earlier
`train_test_split` unpack in the same scope.

## Refit results from the sample

The wheel built from this worktree was installed with pandas and scikit-learn in
`/var/tmp/xpd-refit-venv`. Each copy below ran the documented `execute` script through
`xpd mcp --code-mode --root`. The clean copy passed all 15 gates. The refit read 344 raw
rows, fitted on 258 and compared the 86 held out rows. The fold scores of its clone were
`[0.9807692307692307, 0.9807692307692307, 1.0, 1.0, 0.9803921568627451]`.

| Copy | Gates that fail | Largest probability gap | Held out score in contract | Held out score from refit |
| --- | --- | --- | --- | --- |
| Clean | None | 0.0 | 0.9883720930232558 | 0.9883720930232558 |
| Scaled before the split | `refit-matches-model`, `refit-matches-scores`, `saved-model-predicts` | 0.9999965987620804 | 0.9883720930232558 | 0.9883720930232558 |
| Fitted on every row | `refit-matches-model`, `refit-matches-scores` | 0.1839658915622739 | 0.9883720930232558 | 0.9883720930232558 |
| Target copied into body_mass_g | `refit-matches-model`, `refit-matches-scores`, `saved-model-predicts` | 0.9989810090351536 | 1.0 | 0.9883720930232558 |
| Unseeded NumPy noise in inputs | `reproducible`, `refit-matches-model`, `refit-matches-scores`, `saved-model-predicts` | 0.04555828298731365 | 0.9883720930232558 | 0.9883720930232558 |
| Unseeded NumPy noise in C | `reproducible` | 0.0 | 0.9883720930232558 | 0.9883720930232558 |

The `X_hold` copy stopped at lint with `sklearn/tune-on-test`. For the gate report only,
a second diagnostic script removed the early return after lint. The server returned
`check-reports-no-error` as false with `detail` equal to
`train.py:72 sklearn/tune-on-test.`.

The complete sample took 6.528 seconds before the change and 7.568 seconds after it.
Each number is one wall time measurement from starting the client through its returned
report. Both reports passed all their gates. These are sample timings, not a benchmark.

A Pipeline with StandardScaler and LinearRegression on the diabetes data used `predict`.
Its refit gap was 0.0 and its held out R squared score was 0.35940880381777085 on both
sides. A Pipeline with StandardScaler and LinearSVC on iris used `decision_function`.
Its gap was 0.0 and accuracy was 0.9473684210526315 on both sides. Both fold score lists
matched exactly. A contract without the splitter was refused with
`validation.splitter Field required`.

## Where the other parts plug in

The scikit-lego, MAPIE and Category Encoders packs and the language server are in the same
version as this draft.

| Part | Where it meets a run |
| --- | --- |
| The sklego, mapie and category_encoders packs | `check-reports-no-error` counts every error that `check` reports, so their errors fail the gate, and `split-is-seeded` reads their rule about a draw without a seed. A pack that is added later fails `check-reports-no-error` with no change to the server |
| mapie | A model with prediction intervals needs rows for calibration that no step was fitted on. `Split` would get a third row count, and the contract a section for the coverage. A contract may already hold sections that xpd does not know |
| category_encoders | An encoder that reads the target must be a step of the Pipeline. `inputs-are-file-columns` fails when the contract lists an input that the file does not have, or one with another dtype. It passes when code outside the Pipeline writes new values of the same dtype over a column |
| The language server | It shows the issues of `check` while the scripts are written, before any gate runs. A failed `check-reports-no-error` or `dataflow-connects` gate names a file and a line in its `detail`, which the language server could show in the editor |

## What is open

Discovery ranks the words in names, signatures, docstring summaries, rule texts and recipe titles. It answers "what is the thing called that does X" but does not yet answer questions about a method's properties, such as which linear models need no scaling.

| Open item | What the draft does now |
| --- | --- |
| `gates` cannot tell which files a result is about | It trusts the script. A script that lints another file, or hands in an old result, passes. The tools would have to return the paths they ran on, or `gates` would have to run the stages itself |
| The contract still declares some model facts | Refit checks the actual Pipeline and its outputs and scores. It does not compare the declared step list or every recorded library version with the saved object |
| Training runs three times | The repeat check remains because a clone keeps the saved parameters and does not repeat the training script's choices |
| The model needs its code files | The contract lists them under `model.code`, `contract` refuses a contract that names a missing file, and `predict.py` checks them. Nothing finds a file that the training script left out of the list. A feature step from an installed package would need no file |
| No gate judges the score | A model that is no better than guessing passes. A gate could compare the scores with a baseline, and the held out score with the folds |
| The model file is a joblib pickle | Loading it runs code, so load only a file you wrote. The scikit-learn guide names `skops.io` as the safer format. The draft uses joblib because it installs with scikit-learn |
| One file, one random split | The contract has no place for groups, for a held out split by time, or for data from several files |
| No time limit on a tool call | The draft sets no `wall_clock_timeout_secs`, so a training script that hangs keeps `execute` open |
| `fix` in a script | `fix` with `unsafe` asks the user to confirm. How that question travels through `execute` is not tested |
| The scripts run in the project root | `train.py` imports `features` and reads `penguins.csv` by a relative path, so the server's root has to be the folder of the scripts |
| Choosing a model over several separate runs | No check of one run can see choices made over other runs |
| A raw column that stands in for the target | Refitting preserves the raw column, so judging it needs domain knowledge |
| Held out outputs can agree while fitted attributes differ | Refit compares outputs on the held out rows, not all internal values or all possible inputs |
| A validation split can be a legitimate place to choose | The split rule recognizes second and fourth targets, but does not establish the purpose of each split |

## Discovery before execution

This section extends the first draft. The agent first asks `find` about the task and reads
`api` for the returned candidates. It chooses a symbol or recipe and states its reason.
The kinds are data, so the scripts keep each kind rather than assuming two kinds.
The gate counts entries recorded in `candidates.symbols`, `candidates.rules` and
`candidates.recipes`. An entry read directly through `api` is a candidate too, even if
`find` did not return it. Add that inspected entry to its kind's list before the gate.
The gate checks the supplied record, not a session history of tool calls.
For a recipe, `choice.symbols` names the indexed callable symbols the written example uses.
The agent says when a required library is missing and asks before adding it through the
project's dependency manager. Discovery itself works without that library installed.

The agent writes project files with its normal editing tools. FastMCP's default sandbox
cannot import libraries, write files or ask an LLM to choose. The scripts below repeat
discovery for an already written choice and check those files before running anything.
One `execute` call performs each complete check. It does not generate or install code.

```text
Discovery.question.task
Discovery.candidates.{symbols,rules,recipes}[].kind, name, signature, summary, rules
Discovery.choice.name, reason, symbols
Contract.discovery
Proof.discovery = discovery_gate(paths, discovery)
```

`signature` displays parameter names from the source index, without defaults or parameter
kinds. `discovery_gate` reads the files in `paths` and checks direct imported calls in the
modules containing `choice.symbols`. For example, choosing GroupedPredictor checks direct
calls in `sklego.meta`. Import aliases are resolved by spelling, without Python scope or
type resolution. Each chosen symbol must occur as a call. Each literal keyword must occur
in the indexed parameter names. Unknown callable names and `**` arguments fail the gate.

It does not check instance methods, runtime aliases, shadowing, required arguments,
positional binding, argument values, dependency installation or calls in other modules.
Valid APIs absent from the index also fail in the checked modules. The gate does not prove
that a call will run. `gates` trusts supplied reports and does not check whether source
files changed after the gate. `discovery-is-recorded` compares the discovery record in the
contract with `Proof.discovery`; it does not check that the choice was the best one.

`samples/pipeline/discovery.json` is the inspected planning record for the penguin model.
The training scripts copy it into `model/contract.json`. After changing that choice, save
the new inspected record before running training. The earlier pipeline script returns
fifteen gates, including the API gate and the recorded discovery gate. Contracts and proofs
with neither discovery field use the thirteen gates. Every contract now records
`validation.splitter`.

### Run the scripts through the server

Build and install the wheel and the sample libraries into a throwaway environment. These
commands were run from this worktree, without changing the project's dependencies.
PyArrow and bottleneck satisfy the optional pandas package checks in this environment.

```sh
uv venv /var/tmp/xpd-grouped-verification/venv
uv pip install --python /var/tmp/xpd-grouped-verification/venv/bin/python maturin
/var/tmp/xpd-grouped-verification/venv/bin/maturin build --release --out /var/tmp/xpd-grouped-verification/wheels
uv pip install --python /var/tmp/xpd-grouped-verification/venv/bin/python /var/tmp/xpd-grouped-verification/wheels/*.whl pandas scikit-learn scikit-lego arch mapie plotly pyarrow bottleneck
```

Paste one complete script below into `CODE` in this client and run it from the repository
with `/var/tmp/xpd-grouped-verification/venv/bin/python`. `find` returns an object with `symbols`, `rules` and `recipes` lists directly. `api` returns an object directly.

```python
import asyncio
import json
from fastmcp import Client
from fastmcp.client.transports import StdioTransport

CODE = """paste one script below here"""

async def main():
    transport = StdioTransport(
        "/var/tmp/xpd-grouped-verification/venv/bin/xpd",
        ["mcp", "--code-mode", "--root", "samples/pipeline"],
        env={"VIRTUAL_ENV": "/var/tmp/xpd-grouped-verification/venv"},
    )
    async with Client(transport) as client:
        result = await client.call_tool("execute", {"code": CODE})
        print(json.dumps(result.structured_content, indent=2))

asyncio.run(main())
```

The following outputs were returned by the installed wheel's server through a real
FastMCP 4.2.0 client. `check` selects the Plotly pack because `figure.py` imports Plotly.
The grouped and bootstrap examples pass the API gate, have no lint errors, and have equal `result` values
from two processes. They do not claim the saved model gates of the penguin workflow.

### One model per group

```python
TASK = 'going from one model on all data to one model per group, 24 groups'
CHOICE = 'sklego.meta.GroupedPredictor'
REASON = 'Fit one estimator per group instead of 24 handwritten fit loops.'
SYMBOLS = ['sklego.meta.GroupedPredictor']
ARCH = False
FILE = 'grouped.py'
found = await call_tool("find", {"task": TASK})
inspected = {
    kind: [await call_tool("api", {"name": item["name"]}) for item in found[kind]]
    for kind in ("symbols", "rules", "recipes")
}
if ARCH:
    blocks = await call_tool("find", {"task": "block bootstrap", "package": "arch"})
    for kind in ("symbols", "rules", "recipes"):
        for item in blocks[kind]:
            inspected[kind].append(await call_tool("api", {"name": item["name"]}))
discovery = {
    "question": {"task": TASK}, "candidates": inspected,
    "choice": {"name": CHOICE, "reason": REASON, "symbols": SYMBOLS},
}
proof = await call_tool("discovery_gate", {"paths": [FILE], "discovery": discovery})
if not proof["gate"]["passed"]:
    return {"passed": False, "gate": proof["gate"], "ran": False}
lint = await call_tool("check", {"paths": [FILE]})
errors = [issue for issue in lint["issues"] if issue["severity"] == "error"]
if errors:
    return {"passed": False, "lint": lint, "ran": False}
repeat = await call_tool("equiv", {"old": FILE, "new": FILE})
return {
    "passed": repeat["equivalent"],
    "candidates": [{"kind": item["kind"], "name": item["name"]} for kind in inspected for item in inspected[kind]],
    "choice": discovery["choice"], "api_gate": proof["gate"],
    "lint": lint, "repeat": repeat,
}
```

```json
{
  "passed": true,
  "candidates": [
    {
      "kind": "symbol",
      "name": "sklearn.model_selection.LeaveOneGroupOut"
    },
    {
      "kind": "symbol",
      "name": "sklego.meta.GroupedPredictor"
    },
    {
      "kind": "symbol",
      "name": "linearmodels.PanelOLS"
    },
    {
      "kind": "symbol",
      "name": "sklego.meta.GroupedTransformer"
    },
    {
      "kind": "symbol",
      "name": "pandas.DatetimeIndex"
    },
    {
      "kind": "rule",
      "name": "sklearn/cross-val-score-per-metric"
    },
    {
      "kind": "rule",
      "name": "scipy/per-value-scipy-call"
    },
    {
      "kind": "rule",
      "name": "arch/forecast-per-origin"
    },
    {
      "kind": "recipe",
      "name": "hamilton/recipe/transform"
    },
    {
      "kind": "recipe",
      "name": "hamilton/recipe/data-science"
    },
    {
      "kind": "recipe",
      "name": "arch/recipe/volatility-modeling"
    }
  ],
  "choice": {
    "name": "sklego.meta.GroupedPredictor",
    "reason": "Fit one estimator per group instead of 24 handwritten fit loops.",
    "symbols": [
      "sklego.meta.GroupedPredictor"
    ]
  },
  "api_gate": {
    "name": "indexed-api-calls",
    "stage": "discovery",
    "evidence": [
      "api",
      "source"
    ],
    "passed": true,
    "detail": "Checked 1 direct imported calls for indexed callable names and literal keyword names in api's parameter lists. No code was run."
  },
  "lint": {
    "fixed": 0,
    "fixable": 0,
    "issues": []
  },
  "repeat": {
    "equivalent": true,
    "detail": null
  }
}
```

### Time series bootstrap and validation

```python
TASK = 'time series bootstrap for cross validation'
CHOICE = 'mapie.subsample.BlockBootstrap'
REASON = 'Use MAPIE blocks for out of bag validation, arch blocks for sampling uncertainty, and TimeSeriesSplit for forward validation without resampling.'
SYMBOLS = ['mapie.subsample.BlockBootstrap', 'arch.bootstrap.StationaryBootstrap', 'arch.bootstrap.CircularBlockBootstrap', 'arch.bootstrap.MovingBlockBootstrap', 'sklearn.model_selection.TimeSeriesSplit']
ARCH = True
FILE = 'bootstrap.py'
found = await call_tool("find", {"task": TASK})
inspected = {
    kind: [await call_tool("api", {"name": item["name"]}) for item in found[kind]]
    for kind in ("symbols", "rules", "recipes")
}
if ARCH:
    blocks = await call_tool("find", {"task": "block bootstrap", "package": "arch"})
    for kind in ("symbols", "rules", "recipes"):
        for item in blocks[kind]:
            inspected[kind].append(await call_tool("api", {"name": item["name"]}))
discovery = {
    "question": {"task": TASK}, "candidates": inspected,
    "choice": {"name": CHOICE, "reason": REASON, "symbols": SYMBOLS},
}
proof = await call_tool("discovery_gate", {"paths": [FILE], "discovery": discovery})
if not proof["gate"]["passed"]:
    return {"passed": False, "gate": proof["gate"], "ran": False}
lint = await call_tool("check", {"paths": [FILE]})
errors = [issue for issue in lint["issues"] if issue["severity"] == "error"]
if errors:
    return {"passed": False, "lint": lint, "ran": False}
repeat = await call_tool("equiv", {"old": FILE, "new": FILE})
return {
    "passed": repeat["equivalent"],
    "candidates": [{"kind": item["kind"], "name": item["name"]} for kind in inspected for item in inspected[kind]],
    "choice": discovery["choice"], "api_gate": proof["gate"],
    "lint": lint, "repeat": repeat,
}
```

```json
{
  "passed": true,
  "candidates": [
    {
      "kind": "symbol",
      "name": "sklearn.model_selection.TimeSeriesSplit"
    },
    {
      "kind": "symbol",
      "name": "mapie.subsample.BlockBootstrap"
    },
    {
      "kind": "symbol",
      "name": "mapie.regression.TimeSeriesRegressor"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.IndependentSamplesBootstrap"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.optimal_block_length"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.MovingBlockBootstrap"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.CircularBlockBootstrap"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.LocalBlockBootstrap"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.optimal_block_length"
    },
    {
      "kind": "symbol",
      "name": "arch.bootstrap.StationaryBootstrap"
    },
    {
      "kind": "rule",
      "name": "sklego/group-time-series-split-folds"
    },
    {
      "kind": "rule",
      "name": "mapie/shuffled-time-series-split"
    },
    {
      "kind": "rule",
      "name": "sklearn/cross-val-on-fitted-data"
    },
    {
      "kind": "rule",
      "name": "arch/removed-in-arch"
    },
    {
      "kind": "rule",
      "name": "arch/arch-unseeded-draws"
    },
    {
      "kind": "rule",
      "name": "arch/arch-unknown-option"
    },
    {
      "kind": "recipe",
      "name": "sklego/recipe/cross-validator-that-creates-folds-based-on-provided"
    },
    {
      "kind": "recipe",
      "name": "mapie/recipe/generate-a-sampling-method-that-block-bootstraps-the"
    },
    {
      "kind": "recipe",
      "name": "mapie/recipe/plot-nested-cv"
    }
  ],
  "choice": {
    "name": "mapie.subsample.BlockBootstrap",
    "reason": "Use MAPIE blocks for out of bag validation, arch blocks for sampling uncertainty, and TimeSeriesSplit for forward validation without resampling.",
    "symbols": [
      "mapie.subsample.BlockBootstrap",
      "arch.bootstrap.StationaryBootstrap",
      "arch.bootstrap.CircularBlockBootstrap",
      "arch.bootstrap.MovingBlockBootstrap",
      "sklearn.model_selection.TimeSeriesSplit"
    ]
  },
  "api_gate": {
    "name": "indexed-api-calls",
    "stage": "discovery",
    "evidence": [
      "api",
      "source"
    ],
    "passed": true,
    "detail": "Checked 5 direct imported calls for indexed callable names and literal keyword names in api's parameter lists. No code was run."
  },
  "lint": {
    "fixed": 0,
    "fixable": 0,
    "issues": []
  },
  "repeat": {
    "equivalent": true,
    "detail": null
  }
}
```

### A Plotly figure from scratch

```python
TASK = 'build a plotly scatter figure from scratch'
CHOICE = 'plotly.express.scatter'
REASON = 'Build a new scatter figure from rows and check the Plotly pack before execution.'
SYMBOLS = ['plotly.express.scatter']
ARCH = False
FILE = 'figure.py'
found = await call_tool("find", {"task": TASK})
inspected = {
    kind: [await call_tool("api", {"name": item["name"]}) for item in found[kind]]
    for kind in ("symbols", "rules", "recipes")
}
if ARCH:
    blocks = await call_tool("find", {"task": "block bootstrap", "package": "arch"})
    for kind in ("symbols", "rules", "recipes"):
        for item in blocks[kind]:
            inspected[kind].append(await call_tool("api", {"name": item["name"]}))
discovery = {
    "question": {"task": TASK}, "candidates": inspected,
    "choice": {"name": CHOICE, "reason": REASON, "symbols": SYMBOLS},
}
proof = await call_tool("discovery_gate", {"paths": [FILE], "discovery": discovery})
if not proof["gate"]["passed"]:
    return {"passed": False, "gate": proof["gate"], "ran": False}
lint = await call_tool("check", {"paths": [FILE]})
errors = [issue for issue in lint["issues"] if issue["severity"] == "error"]
if errors:
    return {"passed": False, "lint": lint, "ran": False}
repeat = await call_tool("equiv", {"old": FILE, "new": FILE})
return {
    "passed": repeat["equivalent"],
    "candidates": [{"kind": item["kind"], "name": item["name"]} for kind in inspected for item in inspected[kind]],
    "choice": discovery["choice"], "api_gate": proof["gate"],
    "lint": lint, "repeat": repeat,
}
```

```json
{
  "passed": true,
  "candidates": [
    {
      "kind": "symbol",
      "name": "plotly.figure_factory.create_quiver"
    },
    {
      "kind": "symbol",
      "name": "plotly.figure_factory.create_streamline"
    },
    {
      "kind": "symbol",
      "name": "plotly.figure_factory.create_hexbin_map"
    },
    {
      "kind": "symbol",
      "name": "plotly.express.scatter"
    },
    {
      "kind": "symbol",
      "name": "plotly.express.scatter_map"
    },
    {
      "kind": "rule",
      "name": "plotly/free-facet-scales"
    },
    {
      "kind": "rule",
      "name": "plotly/boxed-axis-frame"
    },
    {
      "kind": "rule",
      "name": "plotly/three-d-for-two-d"
    },
    {
      "kind": "recipe",
      "name": "plotly/recipe/scatter-line-area-and-bar-charts"
    },
    {
      "kind": "recipe",
      "name": "plotly/recipe/figures-as-dictionaries"
    },
    {
      "kind": "recipe",
      "name": "plotly/recipe/setting-subplots-on-a-figure-directly"
    }
  ],
  "choice": {
    "name": "plotly.express.scatter",
    "reason": "Build a new scatter figure from rows and check the Plotly pack before execution.",
    "symbols": [
      "plotly.express.scatter"
    ]
  },
  "api_gate": {
    "name": "indexed-api-calls",
    "stage": "discovery",
    "evidence": [
      "api",
      "source"
    ],
    "passed": true,
    "detail": "Checked 1 direct imported calls for indexed callable names and literal keyword names in api's parameter lists. No code was run."
  },
  "lint": {
    "fixed": 0,
    "fixable": 0,
    "issues": []
  },
  "repeat": {
    "equivalent": true,
    "detail": null
  }
}
```

### An unknown class and argument

```python
TASK = 'going from one model on all data to one model per group, 24 groups'
CHOICE = 'sklego.meta.GroupedPredictor'
REASON = 'Fit one estimator per group.'
SYMBOLS = ['sklego.meta.GroupedPredictor']
ARCH = False
FILE = 'invalid_api.py'
found = await call_tool("find", {"task": TASK})
inspected = {
    kind: [await call_tool("api", {"name": item["name"]}) for item in found[kind]]
    for kind in ("symbols", "rules", "recipes")
}
if ARCH:
    blocks = await call_tool("find", {"task": "block bootstrap", "package": "arch"})
    for kind in ("symbols", "rules", "recipes"):
        for item in blocks[kind]:
            inspected[kind].append(await call_tool("api", {"name": item["name"]}))
discovery = {
    "question": {"task": TASK}, "candidates": inspected,
    "choice": {"name": CHOICE, "reason": REASON, "symbols": SYMBOLS},
}
proof = await call_tool("discovery_gate", {"paths": [FILE], "discovery": discovery})
if not proof["gate"]["passed"]:
    return {"passed": False, "gate": proof["gate"], "ran": False}
lint = await call_tool("check", {"paths": [FILE]})
errors = [issue for issue in lint["issues"] if issue["severity"] == "error"]
if errors:
    return {"passed": False, "lint": lint, "ran": False}
repeat = await call_tool("equiv", {"old": FILE, "new": FILE})
return {
    "passed": repeat["equivalent"],
    "candidates": [{"kind": item["kind"], "name": item["name"]} for kind in inspected for item in inspected[kind]],
    "choice": discovery["choice"], "api_gate": proof["gate"],
    "lint": lint, "repeat": repeat,
}
```

```json
{
  "passed": false,
  "gate": {
    "name": "indexed-api-calls",
    "stage": "discovery",
    "evidence": [
      "api",
      "source"
    ],
    "passed": false,
    "detail": "invalid_api.py:5 sklego.meta.GroupedPredictor has no indexed parameter group_by. invalid_api.py:6 sklego.meta.ImaginaryPredictor is not known to api."
  },
  "ran": false
}
```

### Output from the project scripts

Running the three files directly in `samples/pipeline` with the throwaway environment's
Python printed these lines. The rows of these three examples are made up.

```text
groups=24, models=24, predictions=192, max_error=0.000000
arch_lengths=[48, 48, 48]
bootstrap_folds=3, forward_folds=3
forward_bounds=[(21, 24), (29, 32), (37, 40)]
traces=1, points=3, title=Daily count
```

A separate environment at `/var/tmp/xpd-discovery-bare` had only the wheel and its declared
dependencies. `importlib.util.find_spec("sklego")` was `None`. Its real server still returned
`sklego.meta.GroupedPredictor` with `estimator`, `groups`, `shrinkage`, `use_global_model`,
`check_X` and `shrinkage_kwargs` from one code mode `api` call.

The wheel built from this branch returns grouped discovery results through a real
FastMCP 4.2.0 client. The grouped, bootstrap and Plotly scripts pass the API gate, lint
and repeatability checks. The invented API script stops at the API gate before execution.

A recipe lookup through the same code mode server returned the following inspected fields.

```python
found = await call_tool("find", {"task": "rounded bars", "kind": "recipe"})
entry = await call_tool("api", {"name": found["recipes"][0]["name"]})
return {"name": entry["name"], "kind": entry["kind"],
        "license": entry["recipe"]["license"],
        "source": entry["recipe"]["source"],
        "code_bytes": len(entry["recipe"]["code"])}
```

```json
{"name":"plotly/recipe/rounded-bars","kind":"recipe","license":"MIT","source":"https://github.com/plotly/plotly.py/blob/v7.1.0/doc/python/bar-charts.md#L746","code_bytes":498}
```

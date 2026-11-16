# Code mode

!!! note "First draft"

    This page is a first draft. It describes one way for an agent to take a data file to a
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
    Server->>Python: train.py twice, then train.py and predict.py, each in a new process
    Sandbox->>Server: call_tool("contract"), then call_tool("gates") with every result
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
| One `execute` call makes at most 50 tool calls | The script below makes 7 |
| Prompts and resources are not tools | The agent reads the `build_pipeline` prompt through its client, before it calls `execute` |

## What worked before this draft

Code mode could already chain `profile`, `check`, `graph` and `equiv` in one `execute` call.
Nothing said what a finished model run looks like, and nothing judged the results together.
An agent had to decide for itself whether a run was good.

## What the draft adds

| Addition | Kind | What it does |
| --- | --- | --- |
| The stages of a run | Typed models in `python/xpd/mcp/models.py` | Name what each stage hands to the next |
| `contract` | Tool | Reads the JSON file that the training script saves beside its model |
| `gates` | Tool | Takes the results of the other tools and returns one verdict, with one line per requirement |
| `build_pipeline` | Prompt | Tells the agent which three files to write, and holds the script for `execute` |
| `samples/pipeline/` | Sample project | A run that passes every gate, and a leaky script that fails one |
| `xpd equiv` in two processes | Command | Runs each script in its own Python process, so the prediction script gets no module and no path from the training script |

Neither new tool runs Python or loads a model, and the server needs no new package. The
project's Python needs pandas and scikit-learn.

## The stages

A run has eight stages. Each one hands a typed result to the next.

| Stage | What it hands on | Model | Where it comes from |
| --- | --- | --- | --- |
| Raw data | The file, the target column and the row count | `RawData` | The contract |
| Profile | Each column's dtype, nulls and distinct values | `Profile` | The `profile` tool |
| Split | The seed, the share held out and the rows on each side | `Split` | The contract |
| Preprocessing | The raw columns the model takes, and each step of the one Pipeline | `InputColumn`, `FittedStep` | The contract |
| Model | The last step of the Pipeline | `FittedStep` | The contract |
| Validation | One score per fold, and one score for the held out rows | `Validation` | The contract |
| Proof | What `check`, `graph` and `equiv` found | `Proof` | Those three tools |
| Persisted model | The model file, the class of the saved object, the versions that wrote it and the code files it imports | `SavedModel` | The contract |

The contract is one JSON file with those parts. The training script writes it, and it names
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
    "scoring": "accuracy",
    "folds": 5,
    "cv_scores": [0.9808, 0.9808, 1.0, 1.0, 0.9804],
    "test_score": 0.9884
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

In this draft, a run is done when it passes the eleven gates below. `gates` returns each one
with `passed` and a `detail`. The last column names the tools whose results decide the gate.

A gate shows only what its column says. No gate shows that a run has no leak:
`check-reports-no-error` passes when `check` has no rule for a leak or does not know the held
out rows by their name. [What is open](#what-is-open) lists leaks that pass every gate.

| Gate | Stage | The run passes when | Decided by |
| --- | --- | --- | --- |
| `data-is-profiled` | Raw data | The contract names the profiled file with its row count, and the target has no nulls | `profile`, `contract` |
| `split-covers-every-row` | Split | The training rows and the held out rows add up to the rows of the file | `profile`, `contract` |
| `split-is-seeded` | Split | `check` reports no draw without a seed in scikit-learn, scikit-lego, MAPIE, Category Encoders, arch or pymoo code, by the rules in the table below | `check`, `contract` |
| `inputs-are-file-columns` | Preprocessing | Each input of the contract has the name and dtype of a column of the file, and none is the target. The values are not compared | `profile`, `contract` |
| `dataflow-connects` | Preprocessing | `graph` reports no error or warning, and each feature step reads only input columns and columns of earlier steps | `graph`, `contract` |
| `one-pipeline` | Model | The saved object is a `sklearn.pipeline.Pipeline` with its steps listed | `contract` |
| `check-reports-no-error` | Proof | `check` reports no error of any pack. The table below names the errors that are leaks | `check` |
| `cross-validated` | Validation | The contract has one score for each of two or more folds. Which rows the folds hold is not checked | `contract` |
| `reproducible` | Proof | `equiv` of the training script with itself is equivalent, so two runs in two Python processes leave an equal `result` | `equiv` |
| `saved-model-predicts` | Persisted model | `equiv` of the training script with the prediction script is equivalent, so a new Python process that loads the model from disk leaves the `result` of the fitted one | `equiv` |
| `versions-recorded` | Persisted model | The contract records Python, scikit-learn and pandas, and its pandas is the one `profile` ran with | `profile`, `contract` |

`check-reports-no-error` counts every error of every pack, and not a list of rules.
`split-is-seeded` reads a list of rules, and the server refuses to start when the binary no
longer has one of them. Its rules are from the six packs in the table below. A draw from another
library, such as `np.random.default_rng()` without a seed, is in no list, and only `reproducible`
can find it.

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
profile = await call_tool("profile", {"file": DATA})
lint = await call_tool("check", {"paths": [TRAIN, PREDICT, FEATURES]})
dataflow = await call_tool("graph", {"paths": [FEATURES]})
reproducible = await call_tool("equiv", {"old": TRAIN, "new": TRAIN})
served = await call_tool("equiv", {"old": TRAIN, "new": PREDICT})
contract = await call_tool("contract", {"file": CONTRACT})
proof = {"lint": lint, "dataflow": dataflow, "reproducible": reproducible, "served": served}
report = await call_tool("gates", {"profile": profile, "contract": contract, "proof": proof})
return {
    "passed": report["passed"],
    "gates": [
        {"name": gate["name"], "passed": gate["passed"], "detail": gate["detail"]}
        for gate in report["gates"]
    ],
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
`penguins.csv` and `target` set to `species`. The first `equiv` call runs `train.py` twice,
which fits the Pipeline and saves the model and the contract. The second runs `train.py` and
`predict.py`. Each script runs in its own Python process, so `predict.py` has to import
`features.py` and load the model by itself. Both scripts leave the class probabilities of the
86 held out rows in `result`. This is what the client printed.

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
      "detail": "258 training rows and 86 held out rows."
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
      "detail": "One Pipeline of FunctionTransformer then ColumnTransformer then LogisticRegression."
    },
    {
      "name": "check-reports-no-error",
      "passed": true,
      "detail": "check reports no error."
    },
    {
      "name": "cross-validated",
      "passed": true,
      "detail": "The contract records accuracy 0.9804 to 1.0 on 5 folds, and 0.9884 as the held out score."
    },
    {
      "name": "reproducible",
      "passed": true,
      "detail": "Two runs of the training script, each in its own Python process, leave an equal `result`."
    },
    {
      "name": "saved-model-predicts",
      "passed": true,
      "detail": "The prediction script, in its own Python process, leaves the `result` that the training script left."
    },
    {
      "name": "versions-recorded",
      "passed": true,
      "detail": "python 3.15.2, scikit-learn 1.9.1, pandas 3.1.1, numpy 2.5.5, joblib 1.6.0"
    }
  ],
  "validation": {
    "scoring": "accuracy",
    "folds": 5,
    "cv_scores": [
      0.9808,
      0.9808,
      1.0,
      1.0,
      0.9804
    ],
    "test_score": 0.9884
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

## A leaky script fails a gate

`leaky.py` is `train.py` with one change. It tries four strengths of regularization and keeps
the one with the best score on the held out rows, so that score no longer says how the model
does on rows it has not seen.

```python
best_score, best_strength = 0.0, 1.0
for strength in (0.01, 0.1, 1.0, 10.0):  # xpd: tune-on-test
    candidate = pipeline(strength=strength)
    candidate.fit(X_train, y_train)
    score = candidate.score(X_test, y_test)
    if score > best_score:
        best_score, best_strength = score, strength
model = pipeline(strength=best_strength)
```

The same client with `TRAIN = "leaky.py"` fails `check-reports-no-error`, and the other ten
gates still pass. This is `diff -u` of the two results. The fold scores differ too, because the
leak chose another strength.

```diff
@@ -1,5 +1,5 @@
 {
-  "passed": true,
+  "passed": false,
   "gates": [
     {
       "name": "data-is-profiled",
@@ -33,13 +33,13 @@
     },
     {
       "name": "check-reports-no-error",
-      "passed": true,
-      "detail": "check reports no error."
+      "passed": false,
+      "detail": "leaky.py:72 sklearn/tune-on-test."
     },
     {
       "name": "cross-validated",
       "passed": true,
-      "detail": "The contract records accuracy 0.9804 to 1.0 on 5 folds, and 0.9884 as the held out score."
+      "detail": "The contract records accuracy 0.9423 to 1.0 on 5 folds, and 0.9884 as the held out score."
     },
     {
       "name": "reproducible",
@@ -61,11 +61,11 @@
     "scoring": "accuracy",
     "folds": 5,
     "cv_scores": [
-      0.9808,
+      0.9423,
       0.9808,
       1.0,
       1.0,
-      0.9804
+      1.0
     ],
     "test_score": 0.9884
   },
```

`leaky.py` saves its model where `train.py` does, so run the first client again afterwards.

The gate fails because `check` has the rule `tune-on-test` and the script names the held out
rows `X_test`. The same loop with those rows named `X_hold` passes all eleven gates.

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

| Open item | What the draft does now |
| --- | --- |
| `gates` cannot tell which files a result is about | It trusts the script. A script that lints another file, or hands in an old result, passes. The tools would have to return the paths they ran on, or `gates` would have to run the stages itself |
| The contract is the training script's own account | `gates` checks the file, the rows, the columns, the dtypes and the pandas version against `profile`. The steps, the seed and the scores are not checked against the saved model. That needs a command that loads the model in the project's Python |
| No gate finds a leak that `check` has no rule for | Each script below passes all eleven gates. A gate would need the rows that each step was fitted and scored on, from the run itself and not from the script's own account |
| The held out rows under another name | `leaky.py` with `candidate.score(X_hold, y_hold)`. The rule `tune-on-test` knows the rows by names such as `X_test` |
| A step fitted on every row before the split | `penguins[MEASURED] = StandardScaler().fit_transform(penguins[MEASURED])`, or the same with the mean and the standard deviation of pandas, and then `train_test_split` |
| A fit on every row | `cross_val_score(model, X, y, ...)` and `model.fit(X, y)`, and then `model.score(X_test, y_test)`. `cross-validated` reads the scores of the contract and not the rows behind them |
| The target copied into an input | `penguins["body_mass_g"] = penguins[TARGET].map({...})` before the split scores 1.0 on every fold. `inputs-are-file-columns` compares names and dtypes, and not values |
| `equiv` compares what the scripts leave in `result` | The sample leaves class probabilities, so a training run with unseeded NumPy noise fails `reproducible`. A script that leaves only the predicted classes passes with two models that have other coefficients. The saved model files are not compared |
| The model needs its code files | The contract lists them under `model.code`, `contract` refuses a contract that names a missing file, and `predict.py` checks them. Nothing finds a file that the training script left out of the list. A feature step from an installed package would need no file |
| No gate judges the score | A model that is no better than guessing passes. A gate could compare the scores with a baseline, and the held out score with the folds |
| The model file is a joblib pickle | Loading it runs code, so load only a file you wrote. The scikit-learn guide names `skops.io` as the safer format. The draft uses joblib because it installs with scikit-learn |
| One file, one random split | The contract has no place for groups, for a split by time, or for data from several files |
| Training runs three times | Each `equiv` call runs both of its scripts, each in a new Python process. A tool that runs a script once and keeps its result would cut that to one run |
| No time limit on a tool call | The draft sets no `wall_clock_timeout_secs`, so a training script that hangs keeps `execute` open |
| `fix` in a script | `fix` with `unsafe` asks the user to confirm. How that question travels through `execute` is not tested |
| The scripts run in the project root | `train.py` imports `features` and reads `penguins.csv` by a relative path, so the server's root has to be the folder of the scripts |

"""Prompts for the workflows of the xpd skill. Each names the tools to run and when it is done."""

import json

from fastmcp import FastMCP

from xpd.mcp.catalog import PACK_URI, RULE_URI
from xpd.mcp.names import ToolName

DISCOVERY = f"""\
found = await call_tool("{ToolName.FIND}", {{"task": TASK}})
inspected = {{
    kind: [await call_tool("{ToolName.API}", {{"name": item["name"]}}) for item in found[kind]]
    for kind in ("symbols", "rules", "recipes")
}}
discovery = {{
    "question": {{"task": TASK}}, "candidates": inspected,
    "choice": {{"name": CHOICE, "reason": REASON, "symbols": SYMBOLS}},
}}
api_proof = await call_tool("{ToolName.DISCOVERY_GATE}", {{
    "paths": [TRAIN, PREDICT, FEATURES], "discovery": discovery,
}})
if not api_proof["gate"]["passed"]:
    return api_proof
"""

# The stages of a pipeline run as one code mode script. Each result is handed to `gates`.
STAGES = f"""\
profile = await call_tool("{ToolName.PROFILE}", {{"file": DATA}})
lint = await call_tool("{ToolName.CHECK}", {{"paths": [TRAIN, PREDICT, FEATURES]}})
if any(issue["severity"] == "error" for issue in lint["issues"]):
    return {{"passed": False, "lint": lint, "discovery": api_proof}}
dataflow = await call_tool("{ToolName.GRAPH}", {{"paths": [FEATURES]}})
reproducible = await call_tool("{ToolName.EQUIV}", {{"old": TRAIN, "new": TRAIN}})
served = await call_tool("{ToolName.EQUIV}", {{"old": TRAIN, "new": PREDICT}})
contract = await call_tool("{ToolName.CONTRACT}", {{"file": CONTRACT}})
refitted = await call_tool("{ToolName.REFIT}", {{"file": CONTRACT}})
proof = {{
    "lint": lint, "dataflow": dataflow, "refit": refitted,
    "reproducible": reproducible,
    "served": served, "discovery": api_proof,
}}
report = await call_tool("{ToolName.GATES}", \
{{"profile": profile, "contract": contract, "proof": proof}})
return {{
    "passed": report["passed"],
    "gates": [
        {{"name": gate["name"], "passed": gate["passed"], "detail": gate["detail"]}}
        for gate in report["gates"]
    ],
    "refit": refitted,
    "validation": contract["validation"],
    "model": contract["model"],
}}"""


def discovery_task(target: str, task: str = "") -> str:
    """The task that `find` gets, and that the script records as the discovery question."""
    return task or f"predict {target}"


def stages_script(data: str, target: str, task: str = "") -> str:
    """The script for one project: the paths as constants, then the stages."""
    paths = {
        "DATA": data,
        "FEATURES": "features.py",
        "TRAIN": "train.py",
        "PREDICT": "predict.py",
        "CONTRACT": "model/contract.json",
        "TASK": discovery_task(target, task),
        "CHOICE": "sklearn.linear_model.LogisticRegression",
        "REASON": "Class probabilities for the species target.",
    }
    constants = [f"{name} = {json.dumps(path)}" for name, path in paths.items()]
    return "\n".join([*constants, "SYMBOLS = [CHOICE]", DISCOVERY, STAGES])


def register(mcp: FastMCP, review_uri: str) -> None:
    @mcp.prompt(tags={"lint"})
    def lint_and_fix(path: str = ".") -> str:
        """Lint a path with xpd, fix what it reports, and lint again until it is clean.

        Args:
            path: A file or folder inside the project root.
        """
        return f"""Lint {path} with xpd and fix what it finds.

1. Call the `{ToolName.CHECK}` tool with paths ["{path}"].
2. Call the `{ToolName.FIX}` tool with the same paths to write the safe fixes.
3. For each issue left, read the rule with the `{ToolName.RULE}` tool and change the code as its
   example shows. Fix every `error`. Fix a `warning` unless you can say why the code is right
   here, and say so. Don't silence rules.
4. If an issue has an unsafe fix, you may call `{ToolName.FIX}` with `unsafe` true. The user \
is asked to
   confirm. Read the diff afterwards.
5. A `pandas/missing-<package>` warning names a package the project lacks. Ask the user before
   you add it.
6. Call `{ToolName.CHECK}` again. You are done when it reports no error and every warning left has a
   reason."""

    @mcp.prompt(tags={"review"})
    def review(target: str, package: str = "", rule: str = "") -> str:
        """Review a file, a folder or a diff against xpd's rules and the Python review skill.

        Args:
            target: A path inside the project root, or a git range such as main..HEAD.
            package: Look closest at this pack's rules, such as pandas.
            rule: Look closest at this rule, such as chained-assignment.
        """
        focus = ""
        if rule and package:
            focus = (
                f"\nLook closest at the rule {package}/{rule}. "
                f"Read {RULE_URI.format(package=package, rule=rule)} first."
            )
        elif package:
            uri = PACK_URI.format(package=package)
            focus = f"\nLook closest at the {package} rules. {uri} lists them."
        return f"""Review {target} and report what is slow, wrong or hard to maintain.{focus}

1. Read {review_uri} and follow it. It cites a rule for every finding.
2. If {target} is a git range, list the changed .py files with git and review those.
3. Call the `{ToolName.CHECK}` tool on the files. Each issue is a finding. Cite it as
   `<package>/<rule>` and link the rule's `url` from the `{ToolName.RULE}` tool.
4. If a file has a pipeline of steps or Apache Hamilton nodes, call the `{ToolName.GRAPH}` \
tool on it and
   add its errors and warnings as findings.
5. Then read the code for what a linter cannot see, as the skill describes.
6. Report the findings, worst first, each with the file and line, the rule, and the fix. Change
   no file unless the user asks."""

    @mcp.prompt(tags={"refactor"})
    def refactor_data_logic(path: str, goal: str) -> str:
        """Refactor data logic and prove with `equiv` that the result did not change.

        Args:
            path: The script to refactor, inside the project root.
            goal: What the refactor should achieve, such as "replace the row loop".
        """
        return f"""Refactor {path} so that: {goal}

1. If the script reads a data file, call the `{ToolName.PROFILE}` tool on it first, so your \
choices fit
   the real dtypes, nulls and sizes.
2. Copy {path} to a new file beside it, such as {path}.old.py, before you change anything.
   Both files must leave their output in a variable named `result`. Add one if it is missing.
3. Change {path}. Call the `{ToolName.CHECK}` tool on it and fix what it reports.
4. Call the `{ToolName.EQUIV}` tool with old set to the copy and new set to {path}. The \
refactor is done
   only when it returns `equivalent` true. If it returns false, read `detail` and fix the new
   code, not the comparison.
5. Delete the copy, and tell the user what changed and that `{ToolName.EQUIV}` passed."""

    @mcp.prompt(tags={"dataflow"})
    def add_to_dataflow(path: str, change: str) -> str:
        """Add or change a step of step code or a node of Apache Hamilton modules, and check the
        dataflow with `graph` before and after.

        Args:
            path: The module with the steps or nodes, inside the project root.
            change: The step or node to add, move or rename, and what it computes.
        """
        return f"""Make this change to the dataflow in {path}: {change}

1. Call the `{ToolName.GRAPH}` tool on {path} first. For Apache Hamilton, pass every module of the
   Driver. Read `pipelines[].steps` or `nodes` to see what exists.
2. Reuse a column or node that exists instead of computing its formula again.
3. In step code, return `frame.assign(...)` or `frame.join(...)`, and don't write
   `frame["x"] = ...` on the frame a step was given.
4. Make the change, then call `{ToolName.GRAPH}` again. Fix every `error`. Fix a `warning` \
unless you can
   say why the code is right. For Hamilton, a warning about an input one edit from a node name
   is usually a typing error.
5. Call the `{ToolName.CHECK}` tool on {path} and fix what it reports."""

    @mcp.prompt(tags={"pipeline"})
    def build_pipeline(data: str, target: str, task: str = "") -> str:
        """Build a scikit-learn model from a data file, save it with its contract, and show
        with `gates` that `check` reports no error, that a raw training refit agrees, and that
        a new Python process predicts from the saved file.

        Args:
            data: The data file, inside the project root.
            target: The column the model predicts.
            task: The task in plain words, before choosing a library.
        """
        script = stages_script(data, target, task)
        return f"""Build a model that predicts {target} from {data}. Write three scripts in the \
project root, which is where each of them runs, and save the model in the folder model.

First call `{ToolName.FIND}` with the task {discovery_task(target, task)}, then \
`{ToolName.API}` on the candidates.
Keep each candidate's kind, name, signature, summary and guarding rules. Kinds may include
recipes as well as symbols and rules. Choose a candidate and state why. For a recipe, name
its indexed callable symbols too. Set TASK, CHOICE, REASON and SYMBOLS in the script to those
values. The LogisticRegression values below are an example, not a choice for every task.
Check the project's dependency declaration and environment. Say when a chosen library is
not installed. Get permission before adding it through the project's dependency manager.
Never install silently, and do not treat successful discovery as proof of installation.
The sandbox cannot write files or ask an LLM to choose. Write the files with the agent's
normal editing tools before execute. One execute call repeats discovery and checks that
written choice, then checks code and runs the pipeline stages. It does not generate code.

1. Call the `{ToolName.PROFILE}` tool on {data}. Choose the steps from the real dtypes, nulls \
and distinct
   values. The target must have no nulls.
2. Write features.py. It holds the features that read one row at a time, as step code: each
   step returns `frame.assign(...)`, and one function runs them as `frame.pipe(a).pipe(b)`.
   A step reads only input columns and columns of earlier steps, and never {target}.
3. Write train.py.
   - Split once with `train_test_split` and a `random_state`, before anything is fitted.
   - Build one `sklearn.pipeline.Pipeline` that holds every step. Put
     `FunctionTransformer` on the function of features.py first, so the model takes the raw
     columns of {data}.
   - Score it with cross-validation on the training rows, with a seeded splitter.
     Record its class and literal arguments under `validation.splitter`. Save scores without
     rounding. Then fit it on the training rows and score the held out rows once.
     Choose no setting by that score.
   - Record the full discovery question, inspected candidates and choice with its reason
     as `discovery` in the contract. Use the same values passed to discovery_gate. They are
     the planning input for the split, preprocessing and model stages.
   - Save the Pipeline with joblib to model/model.joblib, and save model/contract.json in
     the shape below.
   - List features.py under `code` in the contract's `model`. The saved Pipeline holds the
     function by its module and name, so the model loads only where features.py can be
     imported.
   - Leave what the model says about the held out rows in `result`, with the rows' index: the
     class probabilities as a DataFrame, or the predictions of a regressor as a Series. Two
     models with other fitted values then leave another `result`, where their classes could
     still agree.
4. Write predict.py. It reads model/contract.json and the files the contract names. It
   raises when the input columns or dtypes of {data} differ from the contract, when
   scikit-learn is another version, or when a file under `code` is missing. It repeats the
   split from the contract, loads the model, and leaves the same in `result` as train.py
   does.
5. Run discovery_gate and check before any equiv call. Stop on a failed gate or lint error.
   Run the stages. With code mode on, call `execute` once with the script below. Without it,
   call the same tools in the same order.
6. The run is done when `passed` is true. For a gate that failed, read its `detail`, fix the
   scripts and run the stages again. Never edit the contract by hand.
7. Refit checks the saved model against raw training rows. Review choices across separate
   runs and raw columns that stand in for the target, since those need other evidence.

The contract, with the values of the run in place of these:

```json
{{
  "data": {{"file": "{data}", "target": "{target}", "rows": 344}},
  "split": {{"seed": 0, "test_size": 0.25, "stratified": true, "train_rows": 258, "test_rows": 86}},
  "inputs": [{{"column": "body_mass_g", "dtype": "float64"}}],
  "steps": [{{"name": "features", "estimator": "FunctionTransformer"}}],
  "validation": {{
    "scoring": "accuracy", "folds": 5,
    "splitter": {{"class": "StratifiedKFold",
                 "args": {{"n_splits": 5, "shuffle": true, "random_state": 0}}}},
    "cv_scores": [0.98], "test_score": 0.99
  }},
  "model": {{
    "file": "model/model.joblib",
    "type": "sklearn.pipeline.Pipeline",
    "versions": {{"python": "3.13.1", "scikit-learn": "1.8.0", "pandas": "3.0.0"}},
    "code": ["features.py"]
  }}
}}
```

The script for `execute`:

```python
{script}
```"""

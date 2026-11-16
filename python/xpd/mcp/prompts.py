"""Prompts for the workflows of the xpd skill. Each names the tools to run and when it is done."""

import json

from fastmcp import FastMCP

# The stages of a pipeline run as one code mode script. Each result is handed to `gates`.
STAGES = """\
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
}"""


def stages_script(data: str) -> str:
    """The script for one project: the paths as constants, then the stages."""
    paths = {
        "DATA": data,
        "FEATURES": "features.py",
        "TRAIN": "train.py",
        "PREDICT": "predict.py",
        "CONTRACT": "model/contract.json",
    }
    constants = [f"{name} = {json.dumps(path)}" for name, path in paths.items()]
    return "\n".join([*constants, STAGES])


def register(mcp: FastMCP) -> None:
    @mcp.prompt(tags={"lint"})
    def lint_and_fix(path: str = ".") -> str:
        """Lint a path with xpd, fix what it reports, and lint again until it is clean.

        Args:
            path: A file or folder inside the project root.
        """
        return f"""Lint {path} with xpd and fix what it finds.

1. Call the `check` tool with paths ["{path}"].
2. Call the `fix` tool with the same paths to write the safe fixes.
3. For each issue left, read the rule with the `rule` tool and change the code as its
   example shows. Fix every `error`. Fix a `warning` unless you can say why the code is right
   here, and say so. Don't silence rules.
4. If an issue has an unsafe fix, you may call `fix` with `unsafe` true. The user is asked to
   confirm. Read the diff afterwards.
5. A `missing-<package>` warning names a package the project lacks. Ask the user before you add
   it.
6. Call `check` again. You are done when it reports no error and every warning left has a
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
                f"Read xpd://rules/{package}/{rule} first."
            )
        elif package:
            focus = f"\nLook closest at the {package} rules. xpd://rules/{package} lists them."
        return f"""Review {target} and report what is slow, wrong or hard to maintain.{focus}

1. Read skill://python-review/SKILL.md and follow it. It cites a rule for every finding.
2. If {target} is a git range, list the changed .py files with git and review those.
3. Call the `check` tool on the files. Each issue is a finding. Cite it as
   `<package>/<rule>` and link the rule's `url` from the `rule` tool.
4. If a file has a pipeline of steps or Apache Hamilton nodes, call the `graph` tool on it and
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

1. If the script reads a data file, call the `profile` tool on it first, so your choices fit
   the real dtypes, nulls and sizes.
2. Copy {path} to a new file beside it, such as {path}.old.py, before you change anything.
   Both files must leave their output in a variable named `result`. Add one if it is missing.
3. Change {path}. Call the `check` tool on it and fix what it reports.
4. Call the `equiv` tool with old set to the copy and new set to {path}. The refactor is done
   only when it returns `equivalent` true. If it returns false, read `detail` and fix the new
   code, not the comparison.
5. Delete the copy, and tell the user what changed and that `equiv` passed."""

    @mcp.prompt(tags={"dataflow"})
    def add_to_dataflow(path: str, change: str) -> str:
        """Add or change a step of step code or a node of Apache Hamilton modules, and check the
        dataflow with `graph` before and after.

        Args:
            path: The module with the steps or nodes, inside the project root.
            change: The step or node to add, move or rename, and what it computes.
        """
        return f"""Make this change to the dataflow in {path}: {change}

1. Call the `graph` tool on {path} first. For Apache Hamilton, pass every module of the
   Driver. Read `pipelines[].steps` or `nodes` to see what exists.
2. Reuse a column or node that exists instead of computing its formula again.
3. In step code, return `frame.assign(...)` or `frame.join(...)`, and don't write
   `frame["x"] = ...` on the frame a step was given.
4. Make the change, then call `graph` again. Fix every `error`. Fix a `warning` unless you can
   say why the code is right. For Hamilton, a warning about an input one edit from a node name
   is usually a typing error.
5. Call the `check` tool on {path} and fix what it reports."""

    @mcp.prompt(tags={"pipeline"})
    def build_pipeline(data: str, target: str) -> str:
        """Build a scikit-learn model from a data file, save it with its contract, and prove
        with `gates` that the run has no leakage, repeats, and predicts from the saved file.

        Args:
            data: The data file, inside the project root.
            target: The column the model predicts.
        """
        script = stages_script(data)
        return f"""Build a model that predicts {target} from {data}. Write three scripts in the \
project root, which is where each of them runs, and save the model in the folder model.

1. Call the `profile` tool on {data}. Choose the steps from the real dtypes, nulls and distinct
   values. The target must have no nulls.
2. Write features.py. It holds the features that read one row at a time, as step code: each
   step returns `frame.assign(...)`, and one function runs them as `frame.pipe(a).pipe(b)`.
   A step reads only input columns and columns of earlier steps, and never {target}.
3. Write train.py.
   - Split once with `train_test_split` and a `random_state`, before anything is fitted.
   - Build one `sklearn.pipeline.Pipeline` that holds every step. Put
     `FunctionTransformer` on the function of features.py first, so the model takes the raw
     columns of {data}.
   - Score it with cross-validation on the training rows, with a seeded splitter. Then fit it
     on the training rows and score the held out rows once. Choose no setting by that score.
   - Save the Pipeline with joblib to model/model.joblib, and save model/contract.json in
     the shape below.
   - Leave the predictions for the held out rows in `result`, as a Series with the rows'
     index.
4. Write predict.py. It reads only model/contract.json. It raises when the input columns or
   dtypes of {data} differ from the contract, or when scikit-learn is another version. It
   repeats the split from the contract, loads the model, and leaves the predictions for the
   held out rows in `result`, as train.py does.
5. Run the stages. With code mode on, call `execute` once with the script below. Without it,
   call the same tools in the same order.
6. The run is done when `passed` is true. For a gate that failed, read its `detail`, fix the
   scripts and run the stages again. Never edit the contract by hand.

The contract, with the values of the run in place of these:

```json
{{
  "data": {{"file": "{data}", "target": "{target}", "rows": 344}},
  "split": {{"seed": 0, "test_size": 0.25, "stratified": true, "train_rows": 258, "test_rows": 86}},
  "inputs": [{{"column": "body_mass_g", "dtype": "float64"}}],
  "steps": [{{"name": "features", "estimator": "FunctionTransformer"}}],
  "validation": {{"scoring": "accuracy", "folds": 5, "cv_scores": [0.98], "test_score": 0.99}},
  "model": {{
    "file": "model/model.joblib",
    "type": "sklearn.pipeline.Pipeline",
    "versions": {{"python": "3.13.1", "scikit-learn": "1.8.0", "pandas": "3.0.0"}}
  }}
}}
```

The script for `execute`:

```python
{script}
```"""

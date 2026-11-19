Each prompt gives the agent the steps of one workflow of the xpd skill, with the tools to call and
the result that ends it. The server completes the `package` and `rule` arguments as it does in
resource templates.

## lint_and_fix

The agent lints, fixes, and lints again until only explained warnings are left. With
`path` set to `samples/bad.py`, the prompt starts like this.

```text
Lint samples/bad.py with xpd and fix what it finds.

1. Call the `check` tool with paths ["samples/bad.py"].
2. Call the `fix` tool with the same paths to write the safe fixes.
```

## review

The agent reviews a path or a git range with the Python review skill and the `check` tool.

## refactor_data_logic

The agent refactors a script and proves with the `equiv` tool that its result did not change.

## add_to_dataflow

The agent adds or changes a step or a node, and calls the `graph` tool before and after.

## build_pipeline

The agent builds a scikit-learn model from a data file and saves it with its contract. It
chooses a library with `find`, `api` and `discovery_gate` before it writes code, and asks the
user before it adds a dependency. It runs the stages until the `gates` tool returns
`passed: true`. With code mode on, one `execute` call chains the stages.

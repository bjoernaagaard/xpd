Each prompt gives the agent the steps of one workflow of the xpd skill, with the commands to run,
the tools to call and the result that ends it. The server completes the `package` and `rule` arguments as it does in
resource templates.

## lint_and_fix

The agent lints, fixes, and lints again until only explained warnings are left. With
`path` set to `samples/bad.py`, the prompt starts like this.

```text
Lint samples/bad.py with xpd and fix what it finds.

1. Run `xpd check samples/bad.py --format json`.
2. Run `xpd check samples/bad.py --fix` to write the safe fixes.
```

## review

The agent reviews a path or a git range with the Python review skill and `xpd check`.

## refactor_data_logic

The agent refactors a script and proves with `xpd equiv --since HEAD` that its result did not
change.

## add_to_dataflow

The agent adds or changes a step or a node, and runs `xpd graph` before and after.

## build_pipeline

The agent builds a scikit-learn model from a data file and saves it with its contract. It
chooses a library with `xpd find`, `xpd api` and the `discovery_gate` tool, and asks the user
before it adds a dependency. It calls the `gates` tool, which runs the stages, until it returns
`passed: true`.

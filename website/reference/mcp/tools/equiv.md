Use `equiv` to prove that a refactor of data code keeps its result, as
[`xpd equiv`](../../../commands/equiv/) does. It runs the two scripts with the project's Python, each in its
own process, and compares the `result` that each one leaves. A refactor is done when it returns
`equivalent: true`. The scripts run as they are, so they can do whatever a Python script does.

## Example

```json
{"old":"samples/equiv/old.py","new":"samples/equiv/wrong.py"}
```

```json
{"equivalent":false,"detail":"DataFrame.iloc[:, 2] (column name=\"w\") are different\n\nDataFrame.iloc[:, 2] (column name=\"w\") values are different (100.0 %)\n[index]: [0, 1, 2]\n[left]:  [2, 4, 6]\n[right]: [3, 6, 9]"}
```

## Errors

A script that fails, exits, or leaves no `result` that pickle can save makes the call fail with
the script's error. So does a project Python without pandas.

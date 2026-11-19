Use `xpd equiv` to check a refactor of data code. It runs two scripts and tells you whether the
variable `result` that each one leaves is equal. DataFrames and Series are compared with
`pandas.testing`, and anything else with `numpy.testing`, so a list and an array with the same
values count as equal.

## Examples

```console
$ xpd equiv samples/equiv/old.py samples/equiv/new.py
{"equivalent": true}
$ xpd equiv samples/equiv/old.py samples/equiv/wrong.py
{"equivalent": false, "detail": "DataFrame.iloc[:, 2] (column name=\"w\") are different\n\nDataFrame.iloc[:, 2] (column name=\"w\") values are different (100.0 %)\n[index]: [0, 1, 2]\n[left]:  [2, 4, 6]\n[right]: [3, 6, 9]"}
```

Each script runs in its own Python process of the project's Python, in the folder where you run
`xpd equiv`. The second script gets no module, no `sys.path` entry and no variable from the
first. What a script prints goes to stderr, so stdout holds only the JSON. xpd gets each `result`
back with Python's `pickle`, so it cannot be an object of a class that the script defines.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | `{"equivalent": true}` |
| 1 | The results differ, with the difference in `detail` |
| 2 | A script failed, exited or left no `result` that pickle can save, or pandas is missing. The JSON has an `error` |

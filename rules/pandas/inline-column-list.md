A list of three or more column names written inside a call is hard to read and to reuse. Give the list a name first and pass the name.

```diff lang="python"
-selected = df[["load", "wind", "solar"]]
+output_columns = ["load", "wind", "solar"]
+selected = df[output_columns]
```

## What the rule reports

The rule reports a list of three or more text literals written inside the call, as in `df[["k", "a", "b"]]` and `df.groupby(["k", "s", "raw"])`, and asks for a named list passed by name. The table shows where the rule looks.

| Place | Reported |
| --- | --- |
| `df[[...]]` and `df.loc[rows, [...]]` | yes |
| an argument of `groupby`, `sort_values`, `set_index`, `select` or `drop_duplicates` | yes |
| `by=`, `subset=`, `on=`, `left_on=`, `right_on=`, `columns=`, `id_vars=`, `value_vars=` or `usecols=` | yes |
| `pd.DataFrame(data, columns=[...])` | no, because the list there defines the frame |

## How to fix it

xpd has no automatic fix, so make the change by hand.

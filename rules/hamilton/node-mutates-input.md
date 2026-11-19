A Hamilton node that writes to its input changes an object that other nodes also receive, so their results depend on which outputs you ask for. Work on a copy, or build a new value.

```diff lang="python"
 def clipped(raw: pd.DataFrame) -> pd.DataFrame:
-    raw.loc[raw["uk"] < 0, "uk"] = 0
-    return raw
+    return raw.assign(uk=raw["uk"].clip(lower=0))
 
 
 def total_uk(raw: pd.DataFrame) -> float:
     return float(raw["uk"].sum())
```

## What the rule reports

The rule reports a node that writes to one of its parameters, such as `raw["uk"] = ...`, `raw.loc[mask, "uk"] = 0`, `raw.columns = ...`, `del raw["dk"]`, `raw.pop("uk")` and `rows.append(...)`.

## How to fix it

xpd has no automatic fix. The copy goes into another statement, at the start of the node or where the second name is bound. Make the change by hand.

Hamilton calls each function once and hands its result, the same object, to every node that
takes it. The result of `total_uk` therefore depends on whether `clipped` ran first. The Hamilton
documentation asks for the same under
[output immutability](https://hamilton.apache.org/concepts/best-practices/output-immutability/).

| Request | Example | Replacement |
| --- | --- | --- |
| `execute(["total_uk"])` | 2.0 | 2.0 |
| `execute(["clipped", "total_uk"])` | 4.0 | 2.0 |

A node is a function at module level with a return hint and a name that does not start with an
underscore. The rule also reports a write through a second name for the input, as in
`gdf = buildings` followed by `gdf["area"] = ...`. It leaves alone the cases in the list:

- A parameter the function rebinds first, as in `raw = raw.copy()`.
- A function under `@load_from`, `@dataloader`, `@mutate` or `@hamilton_exclude`. No other node
  can take the frame that `@load_from` or `@dataloader` passes in.
- `update`, `append` and the like when the result is used, as in `return state.update(row)`,
  because an immutable object returns a new one.
- `inplace=True`, which [no-inplace](../pandas/no-inplace.md) reports.

A write that reaches the input through a helper function is not seen.

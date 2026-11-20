`frame["x"] = ...` changes the frame it is called on, so a function that was given the frame changes the frame of its caller. Return `frame.assign(x=...)`, which builds a new frame.

```diff lang="python"
 def add_total(frame: pd.DataFrame) -> pd.DataFrame:
-    frame["total"] = frame["price"] * frame["qty"]
-    return frame
+    return frame.assign(total=frame["price"] * frame["qty"])
```

## What the rule reports

The rule reports two forms, each with a plain string as the column's name. A name that the code computes, as in `frame[name]` or an f-string, cannot be a keyword of `assign`, so xpd leaves such a write alone.

| Form | Where | What xpd must know |
| --- | --- | --- |
| `frame["x"] = ...` | In a function | `frame` is a parameter annotated `pd.DataFrame`, and nothing assigns it again before the write |
| Three or more such writes in a row to one frame | At the top of a module | The nearest earlier assignment gives the name a frame, such as `pd.read_parquet(...)` or a method of a frame that returns a frame |

A dict is filled the same way, so xpd reports a write only when it can prove that the name holds a frame. It leaves alone a write to a dict, a list, a NumPy array or `self.frame`, and a write to a frame that the function made itself, as after `frame = frame.copy()`. In a run of writes, xpd reports the first one. A cell marker (`# %%`, `# <codecell>` or `# In[1]:`) ends a run, so one write in each cell of a file that runs cell by cell is no run.

With copy-on-write, the function with the write left the new column in the caller's frame. The function with `assign` left the caller's frame as it was and returned an equal frame. At the top of a module, one `assign` gave a frame equal to the one from the writes. A keyword of `assign` can read a column that an earlier keyword of the same call adds, with `pd.col("x")` or a lambda. [`xpd graph`](https://bjoernaagaard.github.io/xpd/commands/graph/) reads a step that returns `assign` and warns about a step that writes in place.

## How to fix it

xpd has no automatic fix, so make the change by hand.

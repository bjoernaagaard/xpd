Growing an array with `np.append` or `np.concatenate` in a loop copies every earlier value on each pass. Collect the pieces in a list and call `np.concatenate` once after the loop.

```diff lang="python"
-out = np.array([])
-for part in parts:
-    out = np.append(out, part)
+out = np.concatenate(parts)
```

## What the rule reports

The rule reports `out = np.append(out, x)`, `out = np.concatenate((out, x))` and the same with `vstack`, `hstack`, `column_stack`, `dstack` and `stack` inside a loop that doesn't reset `out` or read it elsewhere. Each call copies every earlier value.

## How to fix it

xpd has no automatic fix, so make the change by hand.

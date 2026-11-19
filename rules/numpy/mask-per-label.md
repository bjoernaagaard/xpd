Comparing an array with one label per pass makes one pass for every label. Compare with all labels at once, which gives one row per label.

```diff lang="python"
-cleared = np.array([np.where(lot_of == lot, weights, 0.0) for lot in range(lots)])
+cleared = (lot_of == np.arange(lots)[:, None]) * weights
```

## What the rule reports

The rule reports `np.array([... a == k ... for k in ...])`, and a `for` loop over labels that
selects with `values[labels == k]`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The comprehension compares the same array with one label per pass. Broadcasting compares it
with all labels at once and gives one row per label, and on NumPy 2.5.5 the two forms were
equal. For the loop, `pd.DataFrame(values).groupby(labels).median()` reduces every group in one
pass. When the body fits a model or solves a problem for each label, loop over
`frame.groupby(labels)` so that the frame is split once.

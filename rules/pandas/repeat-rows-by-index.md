Repeating rows through `index.repeat` leaves a repeated index to reset and no column for the position. Use a cross merge with a frame of positions.

```diff lang="python"
-long = keys.loc[keys.index.repeat(POSITIONS)].reset_index(drop=True)
-long["position"] = np.tile(np.arange(POSITIONS), len(keys))
+positions = pd.DataFrame({"position": np.arange(POSITIONS)})
+long = keys.merge(positions, how="cross")
```

## What the rule reports

The rule reports `frame.loc[frame.index.repeat(n)]`, and `frame.iloc[np.repeat(...)]`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The repeat copies every row through the index, and the lines after it reset the index and tile
a column for the position inside each copy. A cross merge pairs every row with every position
in one call. On pandas 3.1.1 the two forms gave equal frames.

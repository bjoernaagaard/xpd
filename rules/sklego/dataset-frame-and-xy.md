With `as_frame=True`, the scikit-lego loaders ignore `return_X_y=True` and return one frame with the target in it, so unpacking into X and y raises ValueError. Drop `return_X_y` and take the target column from the frame.

```diff lang="python"
-X, y = load_penguins(return_X_y=True, as_frame=True)
+frame = load_penguins(as_frame=True)
+X, y = frame.drop(columns="species"), frame["species"]
```

## What the rule reports

The rule reports `load_penguins`, `load_arrests`, `load_chicken`, `load_abalone`, `load_heroes` and
`load_hearts` called with both `return_X_y=True` and `as_frame=True`. The loaders test `as_frame`
first and return the whole frame, with the target in it.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Unpacking raised ValueError "too many values to unpack (expected 2)" for all six loaders.
scikit-learn's own loaders return a pair of frames for the same two keywords. `fetch_creditcard`
passes both keywords to `fetch_openml` and is left alone.

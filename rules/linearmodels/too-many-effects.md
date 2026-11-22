In linearmodels, `PanelOLS` takes at most two effects, so three raise ValueError. Use `AbsorbingLS`, which absorbs any number of categorical variables.

```diff lang="python"
-model = PanelOLS(y, X, entity_effects=True, time_effects=True, other_effects=data.occupation)
+effects = data.reset_index()[["nr", "year", "occupation"]].astype("category")
+model = AbsorbingLS(y.reset_index(drop=True), X.reset_index(drop=True), absorb=effects)
```

## What the rule reports

The rule reports `PanelOLS` with `entity_effects=True`, `time_effects=True` and an
`other_effects` that is written as data. It also reports a formula with both effect terms next to
such an `other_effects`. The model takes two effects at most.

## How to fix it

xpd has no automatic fix, so make the change by hand.

7.1 and 7.0 raised `ValueError: At most two effects supported.` for both forms. `AbsorbingLS` with
the three columns as categories ran on 7.1.

With `other_effects=None` the call runs, so the rule counts `other_effects` only when its value is
a subscript, an attribute or a call, such as `data[["occupation"]]`. A name that its function
assigns from such a value and from nothing else counts too. A parameter, a name that holds `None`,
a value read from `self` or from a settings object, and a `get` call are left alone.

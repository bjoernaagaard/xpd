`DictMapper` casts every mapped value to int, so a float loses its fraction without a warning. Map to whole numbers, or map the column with pandas `Series.map` before the pipeline.

```diff lang="python"
-mapper = DictMapper({"low": 0.5, "high": 1.7}, default=-1)
+mapper = DictMapper({"low": 0, "high": 1}, default=-1)
+weights = frame["level"].map({"low": 0.5, "high": 1.7}).fillna(-1)
```

## What the rule reports

The rule reports `DictMapper` with a dict literal that maps to a float with a fraction or to text,
and a `default` that is such a float, text or NaN. `DictMapper` casts every mapped value and the
default to int.

## How to fix it

xpd has no automatic fix, so make the change by hand.

| Mapping | Result | Reported |
| --- | --- | --- |
| `{"a": 0.5, "b": 1.7}` with `default=-1` | 0, 1 and -1, with no warning | yes |
| a text value | ValueError "invalid literal for int() with base 10" | yes |
| `default=np.nan` | ValueError "cannot convert float NaN to integer" | yes |
| a float with no fraction, such as `1.0` | the same value | no |
| `{"a": "1"}` with `default=0`, a string that holds a whole number | 1 | no |

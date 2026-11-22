skfolio AssetPanel distinguishes inactive pairs from missing values for active assets. Its field policy controls what values are valid outside active_mask.

```diff
-FieldCategorical(values=codes, levels=labels, inactive_policy=InactivePolicy.ZERO)
+FieldCategorical(values=codes, levels=labels, inactive_policy=InactivePolicy.MISSING)
```

## What the rule reports

Reserve -1 for missing categorical codes. For numeric fields, use NaN under MISSING, explicitly construct a field with ZERO when zero is meaningful, or use IGNORE when intentionally retaining inactive values. CharacteristicsFactorModel expects the panel as its characteristics input.

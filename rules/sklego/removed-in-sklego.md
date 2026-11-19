These forms were removed from scikit-lego, so the import or the call raises. Use the replacement that the message names.

```diff lang="python"
-from sklego.meta import GroupedEstimator
-from sklego.meta import OutlierRemover
+from sklego.meta import GroupedPredictor
+from sklego.preprocessing import OutlierRemover
 
-frame = load_chicken(give_pandas=True)
-model = DecayEstimator(Ridge(), decay=0.99)
+frame = load_chicken(as_frame=True)
+model = DecayEstimator(Ridge(), decay_kwargs={"decay_rate": 0.99})
```

## What the rule reports

The rule reports forms that scikit-lego removed. An import inside a `try` block that catches the
error is left alone.

| Old form | Removed in | Write instead |
| --- | --- | --- |
| an import of `PatsyTransformer` or its module | 0.8.0 | `FormulaicTransformer`, with no fix |
| an import of `FairClassifier` or its module | 0.8.0 | `DemographicParityClassifier` |
| an import of `GroupedEstimator` or its module | 0.8.0 | `GroupedPredictor` |
| `from sklego.meta import OutlierRemover` | 0.8.0 | `from sklego.preprocessing import OutlierRemover` |
| `give_pandas=` on the dataset loaders | 0.6.6 | `as_frame=` |
| `decay=` on `DecayEstimator` | 0.7.0 | `decay_kwargs={"decay_rate": ...}` |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `as_frame=` for `give_pandas=` on the dataset loaders, which gave equal values with either keyword on scikit-lego 0.6.3. The unsafe fix writes `DemographicParityClassifier` for `FairClassifier`, `GroupedPredictor` for `GroupedEstimator`, and `sklego.preprocessing` for the import of `OutlierRemover` from `sklego.meta` or `sklego.meta.outlier_remover`, because on 0.7.4 each old name is another class than the new one, and `decay_kwargs={"decay_rate": 0.99}` for `decay=0.99`, which no release took beside `decay=` and which gives the same weights only to rounding. The other forms have no fix.

The imports raised ImportError or ModuleNotFoundError, and the keywords raised TypeError.
`PatsyTransformer` has no fix, because `FormulaicTransformer` reads the formula with formulaic and
not with patsy. The weights that `decay=0.97` gave in scikit-lego 0.6 are equal to the weights of
`decay_kwargs={"decay_rate": 0.97}` to rounding, and not bit for bit. Each document carries `since`
with the release that removed its form.

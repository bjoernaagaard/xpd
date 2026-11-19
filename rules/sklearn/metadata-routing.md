These forms work only with metadata routing on, or only with it off, so the call raises RuntimeError or ValueError here. Turn routing on with `sklearn.set_config(enable_metadata_routing=True)`, or use the form for the current setting.

```diff lang="python"
-model = LogisticRegression().set_fit_request(sample_weight=True)
+sklearn.set_config(enable_metadata_routing=True)
+model = LogisticRegression().set_fit_request(sample_weight=True).set_score_request(sample_weight=True)
 cross_validate(model, X, y, params={"sample_weight": w})
```

## What the rule reports

The rule reports forms that raise because metadata routing is off, such as `set_fit_request` and
`Pipeline(transform_input=...)`. It also reports forms that raise because routing is on, such as
`groups=`, `step__sample_weight=` and `params=` with no request in the file.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `params={"groups": groups}` for `groups=` when routing is on, because the old form raises. The other forms have no fix.

xpd applies the replacement with `--fix --unsafe-fixes`.

The rule counts a file as routed when any call in it passes `enable_metadata_routing` with a value
other than `False`. A function or a script that never calls a `set_*_request` method has asked
for nothing.

| Routing | Form | Raises |
| --- | --- | --- |
| off | `set_fit_request` and the other request setters | RuntimeError |
| off | `Pipeline(transform_input=...)` | ValueError at fit |
| on | `groups=` on a cross-validation function | ValueError |
| on | a `step__sample_weight` keyword on `fit` | TypeError |
| on | `params=` when no estimator asked for the metadata | UnsetMetadataPassedError |

On scikit-learn 1.9.1 a weighted `cross_validate` with routing on also needs
`set_score_request`, or it raises for the scorer. Recall was 34 of 34.

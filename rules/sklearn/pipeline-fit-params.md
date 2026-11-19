`Pipeline.fit` raises ValueError on a bare `sample_weight=`, because it takes step parameters as the step name, two underscores and the name. Write it as `<step>__sample_weight=`.

```diff lang="python"
 pipe = make_pipeline(StandardScaler(), LogisticRegression())
-pipe.fit(X, y, sample_weight=w)
+pipe.fit(X, y, logisticregression__sample_weight=w)
```

## What the rule reports

The rule reports `pipe.fit(X, y, sample_weight=w)` on a Pipeline built in the same block, with
metadata routing off.

## How to fix it

xpd has no automatic fix, so make the change by hand.

With routing off, `Pipeline.fit` raises "ValueError: Pipeline.fit does not accept the
sample_weight parameter". The rule reads the Pipeline from an assignment earlier in the same
block, or from the call itself. A file that turns routing on is left alone, because there a plain
name is the right form.

Recall was 23 of 30. The rule found none of 6 `sample_weight=` keywords given to the `fit` of a
search over a Pipeline, which raises the same ValueError. It also misses a Pipeline built outside
a loop and fitted inside it.

These fairness functions return a scorer that takes an estimator, so wrapping them in `make_scorer` or calling them on labels raises and makes every score NaN. Pass the scorer itself to `scoring=`.

```diff lang="python"
-search = GridSearchCV(model, grid, scoring=make_scorer(p_percent_score("sex")))
-fairness = p_percent_score(y_test, y_pred)
+search = GridSearchCV(model, grid, scoring=p_percent_score("sex"))
+fairness = p_percent_score("sex")(model, X_test, y_test)
```

## What the rule reports

The rule reports `make_scorer` around `p_percent_score`, `equal_opportunity_score`,
`correlation_score` or `subset_score`. It also reports `p_percent_score(y_true, y_pred)` and
`equal_opportunity_score(y_true, y_pred)`. Each function takes the column to look at and returns a
scorer with the signature `(estimator, X, y)`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `p_percent_score("sex")` for `make_scorer(p_percent_score("sex"))`. The other forms have no fix.

`make_scorer` calls the scorer with `(y_true, y_pred)`, which the scorer can't take:

| Form | Result |
| --- | --- |
| `make_scorer` around each of the four functions, called | IndexError for three of them, and TypeError for `equal_opportunity_score` |
| the same scorer inside `GridSearchCV` | a best score of NaN, because scikit-learn caught the error |
| `p_percent_score(y, y_pred)` | a function, with no error |
| `p_percent_score("sex")` given to `scoring=` | a score |

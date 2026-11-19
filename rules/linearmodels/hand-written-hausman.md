linearmodels has both Hausman tests built in, so a hand-written one is extra code to keep right. Call the result's `hausman` method or the test that the message names.

```diff lang="python"
-def hausman(fe, re):
-    b = fe.params - re.params
-    v = fe.cov - re.cov
-    return float(b @ np.linalg.inv(v) @ b)
+test = random_effects_result.hausman(fixed_effects_result)
```

## What the rule reports

The rule reports a function whose name starts with `hausman`. It leaves alone `hausman_taylor`,
which is the Hausman and Taylor estimator, and names such as `hausman_table` or `hausman_data`,
which show results or prepare data. 7.1 added the Hausman test of random effects against fixed
effects as a method of the random effects result, and the instrument results have had
`wu_hausman()` for longer.

## How to fix it

xpd has no automatic fix. Needs linearmodels 7.1. Make the change by hand.

On 7.1 the method returned a `HausmanTestResults` with the fields `statistic` and `estimates`, and 7.0 has no such method. The rule reports 25 functions in 25 of the 950 public files. It cannot see what a function tests, so it is info, and the message names both tests.

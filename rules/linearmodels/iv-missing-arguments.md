The linearmodels instrument models take dependent, exog, endog and instruments in that order, so a call with fewer arguments raises TypeError. Pass all four, with `None` for an empty one.

```diff lang="python"
-model = IV2SLS(y, X, Z)
+model = IV2SLS(y, exog, endog, instruments)
```

## What the rule reports

The rule reports an instrument model called with two or three arguments, and the keyword
`instrument=`. statsmodels has an `IV2SLS` of its own, in `statsmodels.sandbox.regression.gmm`,
with the signature `(endog, exog, instrument=None)`, where `endog` is the dependent variable.

## How to fix it

xpd has no automatic fix, so make the change by hand.

7.1 and 7.0 raised `TypeError: IV2SLS.__init__() missing 1 required positional argument: 'instruments'`, and `TypeError: ... got an unexpected keyword argument 'instrument'. Did you mean 'instruments'?` for the keyword. Two of the 950 public files call `IV2SLS(Y, X, Z)` on the class they import from linearmodels. A call that names one of the four as a keyword, or that unpacks its arguments, is left alone.

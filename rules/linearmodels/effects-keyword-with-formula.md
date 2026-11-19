`PanelOLS.from_formula` has no `entity_effects` argument and raises TypeError, because the formula holds the effects. Write `EntityEffects` or `TimeEffects` as a term of the formula.

```diff lang="python"
-model = PanelOLS.from_formula("lwage ~ 1 + exper", data, entity_effects=True)
+model = PanelOLS.from_formula("lwage ~ 1 + exper + EntityEffects", data)
```

## What the rule reports

The rule reports `entity_effects=` or `time_effects=` given to `PanelOLS.from_formula`. The
constructor takes the two keywords, but `from_formula` reads the effects from the formula.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

With the unsafe fix, the term `EntityEffects` or `TimeEffects` is added to a formula written as a string, in place of `entity_effects=True` or `time_effects=True`. The call raised before the fix. A formula that is not a string, a keyword that is not `True`, and a call with both keywords have no fix.

7.1 and 7.0 raised `TypeError: PanelOLS.from_formula() got an unexpected keyword argument 'entity_effects'`. Three of the 950 public files have the form.

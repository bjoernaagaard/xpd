skfolio HRP, HERC and Schur no longer inherit from `BaseHierarchicalOptimization`.

```diff
-isinstance(model, BaseHierarchicalOptimization)
+isinstance(model, (HierarchicalRiskParity, HierarchicalEqualRiskContribution, SchurComplementary))
```

## What the rule reports

Use `BaseOptimization` when the check should include all optimizers. When porting a custom subclass, implement any risk or bound calculations it inherited from the old base.

## How to fix it

Use `xpd check --fix --unsafe-fixes` to replace `BaseHierarchicalOptimization` in a class base list with `BaseOptimization` when `BaseOptimization` is imported. Update `isinstance` and `issubclass` checks by hand.

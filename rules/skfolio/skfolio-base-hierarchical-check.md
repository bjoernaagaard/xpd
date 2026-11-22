skfolio HRP, HERC and Schur no longer inherit from `BaseHierarchicalOptimization`.

```diff
-isinstance(model, BaseHierarchicalOptimization)
+isinstance(model, (HierarchicalRiskParity, HierarchicalEqualRiskContribution, SchurComplementary))
```

## What the rule reports

Use `BaseOptimization` when the check should include all optimizers. When porting a custom subclass, implement any risk or bound calculations it inherited from the old base.

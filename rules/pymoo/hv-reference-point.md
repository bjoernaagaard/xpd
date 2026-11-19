A reference point at the largest value of each objective gives no volume to the solutions that hold those values, so the hypervolume is too low. Use one fixed reference point for every run.

```diff lang="python"
-hv = HV(ref_point=res.F.max(axis=0))(res.F)
+hv = HV(ref_point=np.array([1.1, 1.1]))(res.F)
```

## What the rule reports

The rule reports `ref_point=F.max(axis=0)` and `ref_point=np.max(F, axis=0)` on `HV` and `Hypervolume`, and `HV()` with no argument, which raises AssertionError, "For Hypervolume a reference point needs to be provided!". A reference point at the largest value of each objective lies on the solutions that hold the largest values, so they add no volume.

| Reference point | Hypervolume of three solutions | Of the two extreme solutions alone |
| --- | --- | --- |
| `F.max(axis=0)` | 0.09 | 0.0 |
| the fixed point (1, 1) | 0.37 | not run |

## How to fix it

The severity is warning for a reference point taken from the solutions, error for none. xpd has no automatic fix, so make the change by hand.

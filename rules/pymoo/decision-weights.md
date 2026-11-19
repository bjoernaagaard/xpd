The decision method reads these weights another way than the code means, so the larger weight counts less. Pass `1 / weights`, as the pymoo guide does, when a larger weight should count more.

```diff lang="python"
-i = ASF().do(F, np.array([0.2, 0.8])).argmin()
-j = PseudoWeights(np.array([2, 8])).do(F)
+i = ASF().do(F, 1 / np.array([0.2, 0.8])).argmin()
+j = PseudoWeights(np.array([0.2, 0.8])).do(F)
```

## What the rule reports

The rule reports weights that the decision method reads in another way than the code means.

| Code | How pymoo reads the weights | What it picked |
| --- | --- | --- |
| `ASF().do(F, np.array([0.2, 0.8]))`, with weights written out and not all the same | `ASF` divides each objective by its weight, so the larger weight counts less. The pymoo guide passes `1 / weights`. | the solution (0.1, 0.9), where `1 / weights` picked (0.9, 0.1) |
| a weight above 1 written out in the argument of `PseudoWeights`, such as (2, 8) | `PseudoWeights` compares the weights with shares that sum to 1. | row 0 of five solutions, where (0.2, 0.8) picked row 3 |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

unsafe for ASF: `1 / weights`, which picks another solution, the one the code means when a larger weight stands for a more important objective. None for PseudoWeights.

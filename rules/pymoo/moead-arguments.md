These `MOEAD` arguments raise, because `MOEAD` sets the population size from the reference directions. Drop `pop_size`, and change the number of directions to change the population.

```diff lang="python"
-algorithm = MOEAD(ref_dirs, pop_size=100, decomposition="pbi")
+algorithm = MOEAD(ref_dirs, decomposition=PBI())
```

## What the rule reports

The rule reports two `MOEAD` arguments that raise.

| Argument | What happens |
| --- | --- |
| `pop_size=` | TypeError, "got multiple values for keyword argument 'pop_size'", because `MOEAD` sets the population to the number of reference directions itself since pymoo 0.5.0 |
| `decomposition=` given as a string such as `"pbi"`, the form of pymoo 0.4 | AttributeError in the first generation, "'str' object has no attribute 'do'" |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix drops `pop_size`, and writes `PBI()`, `Tchebicheff()`, `WeightedSum()`, `ASF()` or `PerpendicularDistance()` for the texts `"pbi"`, `"tchebi"`, `"weighted-sum"`, `"asf"` and `"perp_dist"`, in a file that imports that class from its module in `pymoo.decomposition`. The old call raised. `"auto"`, `"aasf"` and other texts have no fix.

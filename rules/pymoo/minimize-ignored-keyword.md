`minimize` stores this keyword on the algorithm and never reads it, so the run stops by the default termination. Pass the stopping rule as `termination`.

```diff lang="python"
-res = minimize(problem, NSGA2(pop_size=100), n_gen=200, random_state=1)
+res = minimize(problem, NSGA2(pop_size=100), termination=("n_gen", 200), seed=1)
```

## What the rule reports

The rule reports a keyword that `minimize` hands to the algorithm, which stores it and reads it nowhere.

| Keyword | What happened |
| --- | --- |
| `n_gen`, `n_eval`, `n_evals`, `n_iter`, `n_max_gen`, `max_gen`, `generations` | With each of the seven, `NSGA2` ran 1,000 generations and 20,000 evaluations, where the default termination stops. |
| `random_state`, `random_seed` | Two runs with either set to 1 returned different fronts, and two runs with `seed=1` returned the same front. |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `termination=("n_gen", 7)` for `n_gen=7`, with `n_gen` for `n_max_gen`, `max_gen` and `generations`, so the run stops where the keyword said. `seed=` for `random_state=` and `random_seed=`. A call that passes a termination, `**` or two stopping keywords has no fix.

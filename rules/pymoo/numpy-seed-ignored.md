Since pymoo 0.6.1.6, pymoo draws from its own generator, so `np.random.seed` does not fix a run. Pass `seed` to `minimize`.

```diff lang="python"
 class MySampling(Sampling):
-    def _do(self, problem, n_samples, **kwargs):
-        return np.random.random((n_samples, problem.n_var))
+    def _do(self, problem, n_samples, random_state=None, **kwargs):
+        return random_state.random((n_samples, problem.n_var))
 
-np.random.seed(1)
-res = minimize(problem, NSGA2(pop_size=100, sampling=MySampling()), ("n_gen", 200))
+res = minimize(problem, NSGA2(pop_size=100, sampling=MySampling()), ("n_gen", 200), seed=1)
```

## What the rule reports

The rule reports `np.random.seed` in a file whose `minimize` gets no `seed`, and a draw from `np.random` inside `_do` of a class that inherits from `Sampling`, `Crossover` or `Mutation`. Since pymoo 0.6.1.6 the library draws from a generator that `minimize` builds from `seed` and passes to each operator as `random_state`.

On pymoo 0.6.1.5 two runs after `np.random.seed(1)` returned the same front, and on 0.6.1.6 and 0.6.2 they returned different fronts. A custom operator gave the results in the table in two runs with `seed=1`.

| Operator | Draws from `np.random` | Draws from `random_state` |
| --- | --- | --- |
| a sampling | different results | the same result twice |
| a crossover | different results | the same result twice |
| a mutation | different results | the same result twice |

## How to fix it

xpd has no automatic fix, so make the change by hand.

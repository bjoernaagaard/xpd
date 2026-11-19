`setup` takes the termination by keyword since pymoo 0.6.0, and a second positional argument raises or is read as `verbose`. Write `termination=`.

```diff lang="python"
 algorithm = NSGA2(pop_size=100)
-algorithm.setup(problem, ("n_gen", 200), seed=1)
+algorithm.setup(problem, termination=("n_gen", 200), seed=1)
```

## What the rule reports

The rule reports a termination given to `setup` as the second positional argument, on a name the file binds to a pymoo algorithm.

| pymoo | `setup(problem, ("n_gen", 7))` |
| --- | --- |
| 0.5.0 and older | the second parameter is the termination |
| 0.6.0 to 0.6.1.5 | TypeError |
| 0.6.1.6 and later | the second parameter is `verbose`, and `NSGA2` printed a line for every generation and was still running after 30 |

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `termination=` before the second argument.

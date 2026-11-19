Drawing from a Generator once per pass of a loop makes one call for every pass. When no pass depends on another, draw all values at once with `size=` and compute on the array.

```diff lang="python"
-errors = []
-for _ in range(units):
-    base = rng.uniform(20, 60)
-    errors.append(base * grid - cost)
+base = rng.uniform(20, 60, size=(units, 1))
+errors = base * grid - cost
```

## What the rule reports

The rule reports `for _ in range(n)` whose body calls a method of a Generator named `rng`, such
as `rng.uniform`, `rng.normal` or `rng.integers`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The loop variable `_` says that no pass depends on its position, so every pass can be drawn in
one call with `size=`, and the arithmetic then runs on an array with one more axis. The numbers
come in another order, so a result that is compared with an earlier run has to be made again.
Keep the loop when a pass reads what an earlier pass wrote.

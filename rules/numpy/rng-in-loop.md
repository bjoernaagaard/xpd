`np.random.default_rng()` inside a loop builds a new Generator on every pass. Create it once before the loop and draw from it.

```diff lang="python"
-noise = [np.random.default_rng().normal() for _ in range(n)]
+rng = np.random.default_rng()
+noise = rng.normal(size=n)
```

## What the rule reports

The rule reports `np.random.default_rng()` without a seed inside a `for` or `while` body or a comprehension. Each call gathers entropy, the random bits for a seed, from the operating system, which takes about 5 microseconds. The rule leaves a loop alone that seeds each pass from its loop variable.

## How to fix it

xpd has no automatic fix, so make the change by hand.

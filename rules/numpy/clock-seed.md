A seed taken from the clock repeats whenever two runs start in the same tick. Let NumPy take a seed from the operating system, and log it when a run must be repeatable.

```diff lang="python"
-rng = np.random.default_rng(int(time.time()))
+rng = np.random.default_rng()
+print(rng.bit_generator.seed_seq.entropy)
```

## What the rule reports

The rule reports a seed taken from `time.time()`, `time.time_ns()`, `time.perf_counter()`, `datetime.now()` and similar clocks. Two runs that start in the same tick of the clock get the same numbers. NumPy recommends a seed from the operating system, and a log of the seed when a run must be repeatable. For `np.random.seed(int(time.time()))` the rule advises the code under `Use instead:`, because `np.random.seed()` has no entropy to log and [`legacy-random`](legacy-random.md) reports it.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.random.default_rng()`, `np.random.RandomState()`, `np.random.SeedSequence()`. The numbers change, and a seed recovered from a logged start time no longer repeats the run. `np.random.seed` has no fix.

`np.random.seed` and the functions beside it use a hidden global state that NumPy keeps only for old code. Create `rng = np.random.default_rng(seed)` once and draw from `rng`.

```diff lang="python"
-np.random.seed(0)
-idx = np.random.choice(len(x), 100, replace=False)
+rng = np.random.default_rng(0)
+idx = rng.choice(len(x), 100, replace=False)
```

## What the rule reports

The rule reports `np.random.seed`, `np.random.rand`, `np.random.randint`, `np.random.randn`, `np.random.choice`, every other distribution function in `np.random`, `np.random.get_state`, `np.random.RandomState(...)` outside files that import scikit-learn, and imports of the same names from `numpy.random`. The message names the Generator method, such as `integers` for `randint`. When the method ran at least 1.5 times faster, the message also gives the speedup in the table.

| Generator method | Faster by |
| --- | --- |
| `random` | 2 times |
| `integers` | 2.5 times |
| `exponential` | 2.4 times |
| `choice(..., replace=False)` | 280 times |

## How to fix it

xpd has no automatic fix. A Generator draws a different stream from the same seed, and `np.random.seed` elsewhere would no longer reach the rewritten call. Make the change by hand.

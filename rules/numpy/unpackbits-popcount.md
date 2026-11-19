Unpacking every byte into eight bits to count the set ones was 37 times slower. Use `np.bitwise_count`.

```diff lang="python"
-ones = np.unpackbits(masks.view(np.uint8)).sum()
+ones = np.bitwise_count(masks).sum()
```

## What the rule reports

The rule reports `np.unpackbits(...).sum()`, `np.sum(np.unpackbits(...))` and `np.count_nonzero(np.unpackbits(...))`. `np.bitwise_count`, added in NumPy 2.0, counts the set bits of each value, and ran 37 times faster on 1,000,000 uint64 values.

## How to fix it

xpd has no automatic fix. Because the view, reshape and axis around unpackbits vary. Make the change by hand.

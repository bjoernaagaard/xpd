`HashingEncoder` hashes every value into 8 columns by default, shared by all encoded columns, so many values collide. Pass `n_components` large enough for your categories.

```diff lang="python"
-encoder = ce.HashingEncoder(cols=["city", "shop"])
+encoder = ce.HashingEncoder(cols=["city", "shop"], n_components=1024)
```

## What the rule reports

The rule reports `HashingEncoder(...)` without `n_components`. The encoder hashes every value into
`n_components` columns, 8 by default, and all encoded columns share them.

## How to fix it

xpd has no automatic fix, so make the change by hand.

| `n_components` | Distinct encoded rows for a column of 100 distinct values |
| --- | --- |
| 8, the default | 8 |
| 64 | 50 |
| 1024 | 94 |

Two columns encoded by one `HashingEncoder()` gave 8 output columns in all, and the rows `(x, y)`
and `(y, x)` got the same code. The encoder therefore also loses which column a value came from.
The rule has no fix, because the right number depends on how many distinct values the columns
hold. A call that passes the number by position, a call inside `isinstance`, and
`HashingEncoder().hashing_trick(...)`, which takes the number as `N`, are left alone.

Other advice about the class was measured and dropped:

- With the default number of threads, `max_process=1` was 23 times faster than the default at
  1,000 rows and 2.2 times slower at 100,000 rows, so no setting is right for both.
- `FeatureHasher` of scikit-learn was 23 times faster at 10,000 rows and 5.4 times faster at
  100,000 rows with the default number of threads, but it returns a sparse matrix and takes no
  column names, so it can't take the place of the encoder.

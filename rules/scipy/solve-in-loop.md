`spsolve` or `solve` inside a loop factors the same matrix again on every pass. Factor it once before the loop with `splu`, then call `lu.solve` in the loop.

```diff lang="python"
-xs = [spsolve(A, b) for b in sides]
+lu = splu(A.tocsc())
+xs = [lu.solve(b) for b in sides]
```

## What the rule reports

The rule reports `scipy.sparse.linalg.spsolve(A, b)` and `scipy.linalg.solve(A, b)` inside a
comprehension or a loop that doesn't assign `A` or take it as its loop name. Each call factors
`A` again, and factoring once was faster:

| Matrix | Right sides | Factoring once was faster by |
| --- | --- | --- |
| sparse, 10,000 rows | 100 | 17 to 20 times |
| dense, 1,000 by 1,000 | 20 | 18 to 23 times |

## How to fix it

xpd has no automatic fix, so make the change by hand.

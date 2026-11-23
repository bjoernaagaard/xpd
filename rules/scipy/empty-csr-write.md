Writing into a CSR or CSC matrix that started empty warns, raises under `python -W error`, and copies the whole matrix each time. Build it as `lil_array`, `dok_array` or from coordinates, then convert once.

```diff lang="python"
-A = sparse.csr_matrix((n, n))
+A = sparse.lil_array((n, n))
 for i, j, v in triples:
     A[i, j] = v
+A = A.tocsr()
```

## What the rule reports

The rule reports a write into a `csr_matrix`, `csc_matrix`, `csr_array` or `csc_array` built from
a shape alone, as in `A = csr_matrix((n, m))` followed by `A[i, j] = v`, `A[i, j] += v` or a slice
write, in a function that doesn't assign `A` again. Each write adds a stored value and copies the
whole matrix. On SciPy 1.18.1 a write emits `SparseEfficiencyWarning`, which raises under
`python -W error`. Moreover, 1,000 writes ran 30 times slower than the same writes into a
`lil_matrix`.

## How to fix it

`xpd check --fix --unsafe-fixes` rewrites module-qualified empty `csr_matrix`/`csc_matrix` constructors to `lil_matrix` and `csr_array`/`csc_array` constructors to `lil_array`. Bare imported names remain manual, and you can convert back with `.tocsr()` or `.tocsc()` once writes finish.

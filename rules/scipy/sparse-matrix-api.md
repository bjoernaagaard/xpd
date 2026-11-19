SciPy expects to deprecate the sparse matrix classes. Use the sparse array classes in new code, and check the code that uses them, because `*` works elementwise on a sparse array.

```diff lang="python"
-X = sparse.csr_matrix(X)
-I = sparse.eye(n)
+X = sparse.csr_array(X)
+I = sparse.eye_array(n)
```

## What the rule reports

The rule reports a call to a sparse matrix class or builder whose SciPy docstring points to the
sparse array API:

| Old call | What the docstring says |
| --- | --- |
| `csr_matrix`, `csc_matrix`, `coo_matrix`, `lil_matrix`, `dok_matrix`, `bsr_matrix`, `dia_matrix` | the sparse matrix interface is expected to be deprecated |
| `eye`, `identity`, `diags`, `spdiags`, `rand`, `random`, `bmat` | it encourages `eye_array`, `diags_array`, `dia_array`, `random_array` and `block_array` |

A function that also names a sparse array class, and a branch taken on a condition that mentions
a matrix, are left alone, because such code builds the matrix on purpose.

## How to fix it

xpd has no automatic fix. Because `*` and `**` work elementwise on a sparse array, a row slice of one is 1-D and `todense` gives an ndarray. Skipped on SciPy older than 1.8 for the classes and 1.12 for the builder functions. Make the change by hand.

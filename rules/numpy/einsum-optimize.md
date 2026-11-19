Without `optimize=`, `einsum` loops over every index at once, which was 339 times slower for three 100 by 100 matrices. Add `optimize=True`.

```diff lang="python"
-chain = np.einsum("ij,jk,kl->il", a, b, c)
+chain = np.einsum("ij,jk,kl->il", a, b, c, optimize=True)
```

## What the rule reports

The rule reports `np.einsum` with three or more 2-D operands and a 2-D result, such as `"ij,jk,kl->il"`, and no `optimize=`.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix adds `optimize=True`.

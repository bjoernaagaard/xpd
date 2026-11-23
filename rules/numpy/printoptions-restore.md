Restoring print options by hand leaves them changed when an error is raised. Use `with np.printoptions(...)`, which restores them in every case.

```diff lang="python"
-saved = np.get_printoptions()
-np.set_printoptions(precision=3)
-print(table)
-np.set_printoptions(**saved)
+with np.printoptions(precision=3):
+    print(table)
```

## What the rule reports

The rule reports `np.set_printoptions(**saved)`, which restores options that the code changed earlier. The docs ask for `np.printoptions` as a context manager, which also restores the options when an error is raised.

## How to fix it

With `--unsafe-fixes`, xpd uses `with np.printoptions(...)` when the only intervening statement stores a `repr()` result.

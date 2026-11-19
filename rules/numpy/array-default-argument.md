A NumPy array as a default argument is one array that every call shares, so a write to it shows up in later calls. Default to None and build the array inside the function.

```diff lang="python"
-def add_one(acc=np.zeros(2)):
+def add_one(acc=None):
+    acc = np.zeros(2) if acc is None else acc
     acc += 1
     return acc
```

## What the rule reports

The rule reports `def f(acc=np.zeros(2))` when the function writes into `acc`, because every call shares the one default array.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Writing an array as text with `np.savetxt` and parsing it back is slow and loses the dtype and the shape. Use `np.save` and `np.load`, which were hundreds of times faster.

```diff lang="python"
-np.savetxt("cache.txt", data)
-data = np.loadtxt("cache.txt")
+np.save("cache.npy", data)
+data = np.load("cache.npy")
```

## What the rule reports

The rule reports `np.loadtxt(f)` and `np.genfromtxt(f)` in a file that also calls `np.savetxt(f, ...)`, and `np.fromfile(f)` in a file that also calls `x.tofile(f)`, with the same file argument. `np.save` and `np.load` keep the dtype and the shape.

## How to fix it

xpd has no automatic fix, so make the change by hand.

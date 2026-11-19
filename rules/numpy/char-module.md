`numpy.char` is legacy, and NumPy's docs point to `numpy.strings`. Call the function of the same name in `np.strings`.

```diff lang="python"
-names = np.char.upper(names)
+names = np.strings.upper(names)
```

## What the rule reports

The rule reports a function of `np.char` when `np.strings`, added in NumPy 2.0, has a function of the same name. NumPy's docs call `np.char` legacy.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The safe fix writes `np.strings.upper` and every other name that is the same function in both modules. Unsafe: the comparisons, such as `np.strings.equal`, which do not strip trailing whitespace, and `np.strings.multiply`, which raises a different error on bad input. Unsafe also: `partition` and `rpartition`, because `np.strings` returns a tuple of three arrays.

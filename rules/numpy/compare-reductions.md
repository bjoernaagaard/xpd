`a.all() == b.all()` compares two single booleans, so it is True for many arrays that differ. Use `np.array_equal(a, b)`.

```diff lang="python"
-same = a.all() == b.all()
+same = np.array_equal(a, b)
```

## What the rule reports

The rule reports `a.all() == b.all()` and `np.any(a) == np.any(b)`, which compare two single booleans and not the arrays.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `np.array_equal(a, b)`.

In an `assert`, [`assert-on-arrays`](assert-on-arrays.md) then advises `np.testing.assert_array_equal(a, b)`, which shows the elements that differ.

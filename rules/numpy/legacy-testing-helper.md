`numpy.testing.assert_` is the legacy form of the `assert` statement. Write a plain `assert`, which pytest shows with the compared values, and use `pytest.raises` for exceptions.

```diff lang="python"
-assert_(res.success, res.message)
+assert res.success, res.message
```

## What the rule reports

The rule reports `numpy.testing.assert_`, which NumPy's testing guidelines call the legacy form of the assert statement. It also reports `numpy.testing.assert_raises` and `assert_raises_regex`, where the guidelines prefer `pytest.raises`. The rule reports a bare `assert_raises` only in a file that imports it from `numpy.testing`, because SciPy imports `pytest.raises` under that name.

## How to fix it

`xpd check --fix --unsafe-fixes` applies the fixes of this rule. Check the result, because an unsafe fix can change what the code does.

The unsafe fix writes `assert x` and `assert x, msg`, with each part in parentheses when the call is on more than one line. The statement is skipped under `python -O`, and its message is built only on failure. `assert_raises` has no fix.

A skipped test never runs, so it proves nothing. Fix what makes it fail, or delete it and write down why.

```diff lang="python"
-@pytest.mark.skip(reason="fails on CI")
 def test_total():
-    assert total(ORDERS) == 3.0
+    assert total(ORDERS) == pytest.approx(3.0)
```

## What the rule reports

The rule reports `pytest.mark.skip` and `unittest.skip` in a test file. It also reports
`pytest.mark.xfail` with `strict=False`, which lets the test fail and the run still pass.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A skipped test stays in the suite and reports nothing, so the failure it was written to catch
comes back unseen. Fix what makes the test fail. When the cause can't be found, delete the test
and write down why. The rule leaves `skipif` alone, because it names the platform or version
where the test can't run, and it leaves an `xfail` without `strict=False` alone, because `strict_xfail`
in the project's settings decides whether it can hide a failure.

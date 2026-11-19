A test with a mock checks the mock and not the real code. Run the real code on real input.

```diff lang="python"
-from unittest.mock import patch
-
-
-def test_total():
-    with patch("orders.load", return_value=[{"price": 2.0}]):
-        assert total() == 2.0
+def test_total(tmp_path):
+    path = tmp_path / "orders.csv"
+    path.write_text("price\n2.0\n", encoding="utf-8")
+    assert total(path) == 2.0
```

## What the rule reports

The rule reports an import of `unittest.mock`, `mock` or `pytest_mock`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A test that replaces a function with a mock passes when the mock is called the way the test
expects, whatever the real function does. Run the real code on real input.

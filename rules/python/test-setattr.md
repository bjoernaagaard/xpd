A test that replaces a function with `monkeypatch.setattr` checks the stand-in and not the real code. Run the real code on real input.

```diff lang="python"
-def test_total(monkeypatch):
-    monkeypatch.setattr(orders, "load", lambda path: [{"price": 2.0}])
-    assert orders.total("orders.csv") == 2.0
+def test_total(tmp_path):
+    path = tmp_path / "orders.csv"
+    path.write_text("price\n2.0\n", encoding="utf-8")
+    assert orders.total(path) == 2.0
```

## What the rule reports

The rule reports `monkeypatch.setattr` in a test file. It leaves `monkeypatch.setenv`, `chdir`,
`syspath_prepend` and `setitem` alone, because they give the real code another input and replace
none of it.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`monkeypatch.setattr` is a mock that needs no import. The test passes when the stand-in is called
the way the test expects, whatever the real function does. Give the real function real input
instead, such as a file in `tmp_path`, an environment variable set with `monkeypatch.setenv`, or
a server on localhost.

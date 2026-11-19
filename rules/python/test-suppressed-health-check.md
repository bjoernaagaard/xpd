A property test that turns off a Hypothesis check of correctness can pass while it checks the wrong thing. Make the state inside the test for each input, and drop the suppression.

```diff lang="python"
-@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])
 @given(st.text())
-def test_round_trip(tmp_path, text):
-    path = tmp_path / "note.txt"
-    path.write_text(text, encoding="utf-8")
-    assert path.read_text(encoding="utf-8") == text
+def test_round_trip(text):
+    with tempfile.TemporaryDirectory() as folder:
+        path = Path(folder) / "note.txt"
+        path.write_text(text, encoding="utf-8")
+        assert path.read_text(encoding="utf-8") == text
```

## What the rule reports

The rule reports `suppress_health_check=` in a test file when it names
`HealthCheck.function_scoped_fixture` or `HealthCheck.differing_executors`, or all checks with
`list(HealthCheck)`. It leaves `too_slow`, `filter_too_much` and `data_too_large` alone, because
they are about speed.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Hypothesis runs the test function once for each generated input, but pytest sets up a function
fixture such as `tmp_path` once for the whole test. One input can then see the files that an
earlier input left behind. Make the folder or the object inside the test, so each input gets its
own. The Hypothesis reference says that suppressing these two checks "may result in an unsound
test".

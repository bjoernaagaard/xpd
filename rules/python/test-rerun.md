Rerunning a test until it passes hides why it fails some of the time. Find the cause, or delete the test and write down why.

```diff lang="python"
-@pytest.mark.flaky(reruns=3)
 def test_worker():
     done = threading.Event()
     start(done)
-    assert done.is_set()
+    assert done.wait(timeout=5)
```

## What the rule reports

The rule reports `pytest.mark.flaky` and an import of `flaky` in a test file.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A test that is run again until it passes hides the run where the code was wrong, and it teaches
whoever reads the result to run the suite once more instead of fixing the code. Find why the
test fails some of the time, which is often an order or a wait that the test doesn't control.
When the cause can't be found, delete the test and write down why.

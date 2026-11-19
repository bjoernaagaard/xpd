A test that waits a fixed time is slow and still fails when the wait is too short. Wait for the result itself, such as a join, an event or a poll with a deadline.

```diff lang="python"
 def test_worker():
     done = threading.Event()
     start(done)
-    time.sleep(0.5)
-    assert done.is_set()
+    assert done.wait(timeout=5)
```

## What the rule reports

The rule reports `time.sleep` and `asyncio.sleep` in a test file, but not `asyncio.sleep(0)`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A sleep waits a fixed time and hopes the work is done by then, so the test fails on a slow
machine and wastes the time on a fast one. Wait for the result itself, with a join, an event or
a poll that has a deadline. `asyncio.sleep(0)` hands control to the event loop and waits for
nothing, so the rule leaves it alone.

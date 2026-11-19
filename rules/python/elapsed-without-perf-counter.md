Measuring a duration with `time.time()` uses the system clock, which can be set back. Use `time.perf_counter()`, which only moves forward.

```diff lang="python"
 import time
 
 
 def timed_fit(model: Model, rows: Rows) -> float:
-    start = time.time()
+    start = time.perf_counter()
     model.fit(rows)
-    return time.time() - start
+    return time.perf_counter() - start
```

## What the rule reports

The rule reports `time.time()` minus a name that the same function, or the module, sets from
`time.time()`. It leaves alone a difference with any other value, such as the modified time
of a file, because that value is a time of day.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`time.time()` reads the clock of the system, which a person or a time server can set. The
Python documentation says it can return "a lower value than a previous call if the system clock
has been set back", so a duration measured with it can be wrong and can be negative.
[`time.perf_counter()`](https://docs.python.org/3/library/time.html#time.perf_counter) is
"a clock with the highest available resolution to measure a short duration", and it only moves
forward. Its value means nothing on its own, so keep `time.time()` for a time of day that the
code stores or prints.

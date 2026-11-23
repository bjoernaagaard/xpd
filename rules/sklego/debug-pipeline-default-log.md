`log_callback="default"` logs at level INFO, which Python hides until logging is set up, so the pipeline prints nothing. Call `logging.basicConfig(level=logging.INFO)` first, or pass a callback that prints.

```diff lang="python"
+import logging
+
+logging.basicConfig(level=logging.INFO)
 pipeline = DebugPipeline(steps, log_callback="default")
```

## What the rule reports

The rule reports `log_callback="default"` on `DebugPipeline` and `make_debug_pipeline` in a file
that neither imports `logging` nor uses pytest's `caplog`. The default callback logs each step at
level INFO, which Python hides until logging is set up.

## How to fix it

With `--unsafe-fixes`, xpd uses a callback that prints the step type and elapsed time.

Without the setup, fitting the pipeline printed 0 characters. After
`logging.basicConfig(level=logging.INFO)` it logged one line for each step, such as
`[StandardScaler()] shape=(400, 6) time=0s`. Logging may be set up in another module, so the rule
is info.

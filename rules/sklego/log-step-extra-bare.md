A bare `@log_step_extra` takes the decorated function as a log function, so calling the function returns another function and not a frame. Pass the log functions, as in `@log_step_extra(len)`.

```diff lang="python"
-@log_step_extra
+@log_step_extra(len)
 def add_total(frame):
     return frame.assign(total=frame["price"] * frame["count"])
```

## What the rule reports

The rule reports `@log_step_extra` without arguments, as a bare decorator or with empty
parentheses. `log_step` works both ways, but `log_step_extra` takes the log functions as its
arguments. As a bare decorator it takes the decorated function as a log function.

## How to fix it

xpd has no automatic fix, so make the change by hand.

| Form | Result |
| --- | --- |
| `@log_step_extra` | Calling the function returned another function, not a frame, and raised nothing. |
| `@log_step_extra()` | ValueError "Supply at least one log_function for log_step_extra" |

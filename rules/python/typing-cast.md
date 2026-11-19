`cast()` overrides the type checker without any check at run time. Fix the type where the value is made, or narrow it with `isinstance`, `match` or a `TypeIs` function.

```diff lang="python"
-from typing import cast
-
-
 def port(settings: dict[str, object]) -> int:
-    return cast(int, settings["port"])
+    value = settings["port"]
+    if not isinstance(value, int):
+        raise TypeError(f"port must be an int: {value!r}")
+    return value
```

## What the rule reports

The rule reports a call to `typing.cast`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`cast` tells the type checker to believe a type that nothing checks, so a wrong value fails at
run time, far from the cast. `isinstance`, `match` and a `TypeIs` function check the value in the
program, and the type checker then knows its type.

xpd reports `typing.cast(...)`, and `cast(...)` in a file that imports `cast` from `typing` or
`typing_extensions`. ruff has no rule that reports a cast, and its `TC006` only asks for quotes
around the type.

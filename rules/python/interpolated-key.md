A name built from parts with an f-string, such as an event name or a module path, is in no source file as a whole, so a search for the name seen in a log or a config finds nothing. Write each name out whole.

```diff lang="python"
-def event_name(entity: str, action: str) -> str:
-    return f"github.{entity}.{action}"
+EVENT_NAMES = {
+    ("pr", "merged"): "github.pr.merged",
+    ("pr", "closed"): "github.pr.closed",
+}
+
+
+def event_name(entity: str, action: str) -> str:
+    return EVENT_NAMES[entity, action]
```

## What the rule reports

The rule reports an f-string with no spaces in two places:

- Where fixed words and a dot come before a value, such as `f"set.{key}"` or
  `f"experiments.{name}"`. Event names, flags, settings keys and module paths are written this way.
- As an argument of `getattr`, `setattr`, `hasattr`, `delattr`, `import_module` or `__import__`,
  such as `getattr(module, f"load_{kind}")`. Code that is found by a built name has no caller
  that a search can see.

The rule leaves alone the cases in the list:

- Text with a space, such as a message, which a search finds by its fixed words.
- A path with a slash, such as `f"data/{name}.csv"`.
- A column name with a value first, such as `f"{zone}_mw"`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

Write every name out whole, in a dict or a `match` from the value to its name, or call the
function by its own name. Each full name is then in the source, and a search for it finds the
place that uses it.

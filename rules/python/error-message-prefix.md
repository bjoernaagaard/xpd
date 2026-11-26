An error message that starts with a value has no fixed words at its start, so a line copied from a log does not search back to the raise. Start the message with fixed words, and put the values after them.

```diff lang="python"
 def require_column(columns: list[str], column: str) -> None:
     if column not in columns:
-        raise KeyError(f"{column}: missing from the frame")
+        raise KeyError(f"Column missing from the frame: {column}")
```

## What the rule reports

The rule reports an f-string that starts with a `{...}` value when it is the first argument of the
exception in a `raise` statement.

The rule leaves alone the cases in the list:

- A message held in a variable, such as `raise KeyError(message)`.
- An f-string that starts with fixed words, such as `f"Column missing: {column}"`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A search for a line from a log uses its start, such as `price: missing from the frame`. With the
value first, that text is in no source file, and a search for the fixed part alone also finds
every other message that shares those words. With fixed words first, the start of every message
from this raise is the same text as the source, and one search finds the raise.

ruff's `EM102` asks for the f-string to move into a variable before the raise. It keeps the order
of the parts, so it neither reports nor fixes what this rule reports.

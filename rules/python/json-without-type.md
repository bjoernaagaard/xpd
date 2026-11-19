Returning the result of `json.loads` gives the caller `Any`, so nothing names the shape of the data. Read the fields where you parse them and return a NamedTuple.

```diff lang="python"
 import json
+from typing import NamedTuple
 
 
-def quote(body: bytes) -> dict:
-    return json.loads(body)
+class Quote(NamedTuple):
+    price: float
+
+
+def quote(body: bytes) -> Quote:
+    return Quote(**json.loads(body))
```

## What the rule reports

The rule reports a function that returns the result of `json.loads` or `json.load`, or a key
of that result. It leaves alone a result that the function reads into a typed value first.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`json.loads` returns `Any`, so the type checker accepts every use of the result. When a
function returns it, a wrong key raises far from the place where the data came in, or doesn't
raise at all. The standard library has no decoder that checks the type of each value.
`Quote(**json.loads(body))` raises a `TypeError` on a missing key and on a key that `Quote`
does not have, and the callers get named fields. When the JSON has more keys than the function
needs, read those keys by name and pass them to the `NamedTuple`.

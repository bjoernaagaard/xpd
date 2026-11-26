A function that takes two or more IDs as plain `str` or `int` lets a caller swap them without a type error. Give each kind of ID its own `NewType`.

```diff lang="python"
+from typing import NewType
+
+UserId = NewType("UserId", str)
+OrgId = NewType("OrgId", str)
+
+
-def transfer_owner(user_id: str, org_id: str) -> None: ...
+def transfer_owner(user_id: UserId, org_id: OrgId) -> None: ...
```

## What the rule reports

The rule reports a function with two or more parameters named `id` or ending in `_id`, each typed
`str` or `int`.

The rule leaves alone the cases in the list:

- A parameter typed with a union, such as `org_id: str | None`.
- A file that imports `hamilton`, where Hamilton matches nodes by their declared types.

## How to fix it

xpd has no automatic fix, so make the change by hand.

`transfer_owner(org_id, user_id)` passes the type checker when both IDs are `str`, and the bug
shows only in the data. With a `NewType` for each kind of ID, the checker reports the swap and
names both types. A `NewType` is the plain value at run time, so it costs nothing. Define the
types where the IDs enter the program, such as where a row is read, so every function after that
point takes them.

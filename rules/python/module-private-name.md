A module level name that starts with an underscore hides code that other modules may need. Give it a public name, and list what callers may import in `__all__`.

```diff lang="python"
-_LIMIT = 100
+__all__ = ["total"]
+
+LIMIT = 100
 
 
-def _prices(rows):
-    return [row["price"] for row in rows][:_LIMIT]
+def prices(rows):
+    return [row["price"] for row in rows][:LIMIT]
 
 
 def total(rows):
-    return sum(_prices(rows))
+    return sum(prices(rows))
```

## What the rule reports

The rule reports a function, class or variable at module level whose name starts with an underscore.

The rule leaves alone the cases in the list:

## How to fix it

xpd has no automatic fix, so make the change by hand.

`__all__` says in one place which names callers may import, and `from module import *`, type
checkers and documentation tools all read it. An underscore says the same thing once per name
and makes every use of the name harder to read.

- A method or attribute of a class, such as `def _load(self)`, and a name inside a function.
- `_` alone, and names with two leading underscores such as `__all__` and `__version__`.
- `_T = TypeVar("_T")`, and the same for `ParamSpec` and `TypeVarTuple`. A type variable is
  never part of a module's API, and typeshed, the type stubs of the standard library, and the
  typing documentation write it with an underscore. On Python 3.12, `def first[T](xs: list[T]) -> T` needs no module level name.
- A file that imports `hamilton`, where an underscore tells Hamilton that a function is not a
  node.

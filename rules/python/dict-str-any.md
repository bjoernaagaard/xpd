`dict[str, Any]` in a signature hides which keys and values the function works with. Name the fields in a NamedTuple, or in a TypedDict when the value has to stay a dict.

```diff lang="python"
-from typing import Any
+from typing import NamedTuple
 
 
-def total(order: dict[str, Any]) -> float:
-    return order["price"] * order["quantity"]
+class Order(NamedTuple):
+    price: float
+    quantity: int
+
+
+def total(order: Order) -> float:
+    return order.price * order.quantity
```

## What the rule reports

The rule reports `dict[str, Any]` as the type of a parameter or a return value. `Dict`, `Mapping` and
`MutableMapping` with the same arguments are reported too.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The type says nothing about the keys or the values, so the type checker cannot find a
misspelled key. Every function that takes the dict has to know its keys and the types of its
values. Name the fields once, where the data enters the program, in a `NamedTuple` from the
standard library, with `Order(**row)` for a dict. Use a `TypedDict` when the value has to stay
a dict, as the `**kwargs` of a function and a dict that a library asks for do.

ruff's `ANN401` reports a parameter or a return value annotated with `Any` alone, and leaves
`dict[str, Any]` alone.

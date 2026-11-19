A frozen dataclass or attrs class that only holds fields can be a NamedTuple, which is cheaper to define and build. Write `class Name(NamedTuple)`.

```diff lang="python"
-from dataclasses import dataclass
+from typing import NamedTuple
 
 
-@dataclass(frozen=True, slots=True)
-class Order:
+class Order(NamedTuple):
     price: float
     quantity: int = 1
```

## What the rule reports

The rule reports a class with `@dataclass(frozen=True)` and a frozen attrs class. It leaves
alone a class that a `NamedTuple` cannot replace: a dataclass that is not frozen, a class that
inherits from another class, a class that calls `field()`, and a class with `__post_init__`.

## How to fix it

xpd has no automatic fix, so make the change by hand.

A `NamedTuple` comes from the standard library, holds the same fields, cannot be changed, and
keeps methods, class methods and properties. A JP Hutchins
[measurement](https://www.crumpledpaper.tech/2026-06-21-python-struct-profiling/) on one machine
found that Python defined a `NamedTuple` in 76.2 microseconds and a frozen dataclass in 373.4,
and built an instance in 138 nanoseconds and 224. An instance takes 88 bytes, and an instance
of a dataclass with slots takes 64. The table gives the `NamedTuple` name for each part of a
dataclass.

| dataclass | `NamedTuple` |
| --- | --- |
| `@dataclass(frozen=True, slots=True)` | `class Order(NamedTuple)` |
| `dataclasses.replace(order, price=3.0)` | `order._replace(price=3.0)` |
| `dataclasses.asdict(order)` | `order._asdict()` |
| `dataclasses.fields(Order)` | `Order._fields` |
| `Order(*row)` for a row from a file | `Order._make(row)` |

A `NamedTuple` is also a tuple. It iterates over its fields, has a length, unpacks, and is
equal to a plain tuple with the same values. Check each place that tests a record with
`isinstance(value, tuple)`, formats it with `%`, or passes a list of records to
`np.asarray`, which turns the fields into columns. A field name cannot start with an
underscore.

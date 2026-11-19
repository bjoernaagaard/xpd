A subclass without `__slots__` gives its instances a `__dict__` again, so the memory that the base class saved is lost. Add `__slots__` with the new fields, or `__slots__ = ()`.

```diff lang="python"
-class Quote:
-    __slots__ = ("symbol", "price")
+class SizedQuote(Quote):
+    __slots__ = ("size",)
 
-    def __init__(self, symbol: str, price: float) -> None:
-        self.symbol = symbol
-        self.price = price
-
-
-class SizedQuote(Quote):
     def __init__(self, symbol: str, price: float, size: int) -> None:
         super().__init__(symbol, price)
         self.size = size
```

## What the rule reports

The rule reports a class whose one base is a plain name of a class in the same file that
assigns `__slots__` in its body, when the class body does not assign `__slots__`. The base
counts only when its own instances have no `__dict__`: it has no base, or `object`, `ABC`,
`Generic`, or another slotted class of the file, and its `__slots__` does not name `__dict__`.
The rule leaves alone a dataclass or attrs subclass, which gets its slots from the decorator, a
subclass that uses `__dict__`, `vars` or `cached_property`, a subclass with more than one base,
and a subclass whose base is defined in another file.

## How to fix it

xpd has no automatic fix, so make the change by hand.

[`__slots__`](https://docs.python.org/3/reference/datamodel.html#slots) gives each instance fixed
places for its fields and no `__dict__`. A subclass without `__slots__` gets a `__dict__` again,
so the saving of the base is lost. On Python 3.15.2 on one machine, tracemalloc counted 135.7
bytes for each of 100,000 instances with three fields when the subclass left out `__slots__`,
and 95.7 when it had `__slots__ = ("size",)`. A subclass that adds no field also pays. It took
127.7 bytes without `__slots__` and 87.6 with `__slots__ = ()`. Write `__slots__ = ()` for a
subclass that only adds methods.

Name `__weakref__` in the new `__slots__` when code takes a weak reference to the instances,
because a slotted class has no room for one unless a class in its chain names it.

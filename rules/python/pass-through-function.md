A function that only passes its parameters to one call adds a name without adding behavior. Call the inner function directly, and delete this one.

```diff lang="python"
-def reference(curve: Curve, at_price: float) -> float:
-    return float(curve(at_price))
-
-
-quantity = reference(curve, price)
+quantity = curve(price)
```

## What the rule reports

The rule reports a module level function whose body is one `return` of a call that takes only
the function's parameters as arguments. It also reports a wrapper around that call when the
call is the wrapper's only argument, as in `float(curve(at_price))` when `at_price` is a
parameter. It leaves alone a method, a decorated function and a function that computes
something, such as a formula. It also leaves alone a function that returns a chain of
two or more `pipe` calls, such as `frame.pipe(clean).pipe(features)`, because that function names
the steps of a pipeline in the order they run.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The function gives the call a second name, and a reader has to open it to learn that it does
nothing else. When the function is passed as a value, pass the function it calls.

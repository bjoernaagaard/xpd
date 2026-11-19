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
the function's own names as arguments, also when one more call wraps it, as in
`float(curve(at_price))`. It leaves alone a method, a decorated function and a function that
computes something, such as a formula.

## How to fix it

xpd has no automatic fix, so make the change by hand.

The function gives the call a second name, and a reader has to open it to learn that it does
nothing else. When the function is passed as a value, pass the function it calls.

Each `insert(0, x)` moves every item already in the list, so a loop of them gets slow. Append in the loop and reverse once after it, or use a `collections.deque`.

```diff lang="python"
 def newest_first(events: list[Event]) -> list[Event]:
     found: list[Event] = []
     for event in events:
-        found.insert(0, event)
+        found.append(event)
+    found.reverse()
     return found
```

## What the rule reports

The rule reports `items.insert(0, item)` in the body of a `for` or `while` loop, when the same
function, or the module, starts `items` as `[]` or `list()` under a plain name. It leaves alone
an insert outside a loop, an insert at another index, a list that the function takes as a
parameter, an attribute, and a list that starts with items.

## How to fix it

xpd has no automatic fix, so make the change by hand.

An insert at index 0 moves every item already in the list one place to the right, so the loop
does work that grows with the square of the length of the list. On Python 3.15.2 on one
machine, building a list of 10,000 items took 4.87 milliseconds with `insert(0, item)`, 0.074
with `append` and one `reverse()` after the loop, and 0.103 with
[`collections.deque`](https://docs.python.org/3/library/collections.html#collections.deque) and
`appendleft`. The loop with `insert` took 1.39 times as long at 10 items, 2.06 times at 100 and
6.87 times at 1,000, and it was never faster. The three forms give equal contents.

Keep the `deque` when the loop also reads the front of the collection while it builds it,
because the list is in the opposite order until `reverse()` runs. A `deque` is not a list, so
call `list(found)` where the caller needs one, or slices it.

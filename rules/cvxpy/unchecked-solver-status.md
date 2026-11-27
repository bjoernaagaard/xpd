A standalone `problem.solve()` can leave solution values unavailable. Read
`problem.status` before consuming the problem's value or the value of a
variable it owns. Decide which statuses satisfy the application's tolerances.

The rule follows straight-line code in the same block and stops at a status
read, a rebind or a control-flow join.

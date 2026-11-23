# CVXPY

The CVXPY pack checks problem construction, solve results, and expression
syntax. The examples were verified with CVXPY 1.9.3.

## Problems and solver results

- mutate-problem-constraints
- solve-result-as-status

## Expression construction

- implicit-reshape-order
- star-matmul
- dense-identity-quad-form

## Runtime evidence

| Rule | Observed with CVXPY 1.9.3 | Fix |
| --- | --- | --- |
| [mutate-problem-constraints](mutate-problem-constraints.md) | The three list edits left zero constraints in the problem. A new problem with the added lower bound changed the minimum from 0 to 1. | None |
| [solve-result-as-status](solve-result-as-status.md) | The optimal objective of 0 was false. The infeasible objective of infinity was true. | None |
| [implicit-reshape-order](implicit-reshape-order.md) | Omitted order and literal `None` emitted a FutureWarning and gave the same values as `"F"` for all three APIs. | Safe |
| [star-matmul](star-matmul.md) | A 3 by 3 parameter times a length-3 variable gave the same length-3 result as `@`. The elementwise product was 3 by 3. | None |
| [dense-identity-quad-form](dense-identity-quad-form.md) | Both identity constructors gave a scalar of 14 for `[1, 2, 3]`, as did `sum_squares`. | None |

Rules use imports and local bindings to resolve CVXPY calls. They leave
dynamic shapes, data flow across files, and solver availability to the
application.

[CVXPY API reference](https://www.cvxpy.org/api_reference/cvxpy.html)

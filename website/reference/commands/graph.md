Use `xpd graph` after you add, move or rename a step of a pandas pipeline or a node of an
[Apache Hamilton](https://hamilton.apache.org/) module. It prints which part of the code reads
the result of which other part, and it reports the reads that would fail and the results that
nothing reads. It imports nothing and runs nothing, so it needs no Python.

`graph` reads each file as step code, as Hamilton modules, or both. A file with a function that
passes a frame through other functions of the file, as `frame.pipe(a).pipe(b)` or as
`frame = a(frame)` followed by `frame = b(frame)`, is step code. Any other file is read as the
Hamilton modules of one Driver, so pass all modules of one Driver together.

## Examples

```console
$ xpd graph samples/graph/steps.py --format text
samples/graph/steps.py:50: features: 4 steps, 7 columns
samples/graph/steps.py:18:   (source columns) -> totals  [2 columns]
samples/graph/steps.py:28:   totals -> shares  [1 column]
samples/graph/steps.py:33:   (source columns) -> discounts  [1 column]
samples/graph/steps.py:40:   totals + discounts -> flags  [3 columns]
samples/graph/steps.py:50:   reads from its source: columns=["customer","discount","quantity","unit_price"]
samples/graph/steps.py:35: warning: discounts() computes before_discount with the formula of the column gross_amount from totals(). Read the column gross_amount there.
samples/graph/steps.py:36: warning: discounts() writes to the frame it was given. Called as discounts(orders), it changes the caller's frame too. Return orders.assign(...) or write to orders.copy().
samples/graph/steps.py:44: warning: The column over_limit from flags() has the same formula as is_large from flags(), so the result holds one column twice. Keep one of them.
samples/graph/steps.py:50: warning: The tree in the module docstring lists totals, shares, flags and features() runs totals, shares, discounts, flags. Change the one that is out of date.
xpd: 1 pipeline with 4 steps in 1 file, 4 warnings
```

| Line | Meaning |
| --- | --- |
| `features: 4 steps, 7 columns` | The pipeline, named after its function |
| `totals + discounts -> flags  [3 columns]` | The step `flags` reads columns that `totals` and `discounts` add, and adds 3 columns |
| `(source columns) -> totals` | The step reads only columns of the frame the pipeline starts from |
| `reads from its source: columns=[...]` | The columns that the starting frame must have, written as the `columns=` argument of `pd.read_parquet` |

A step in the wrong order is an error, and the run exits 1.

```console
$ xpd graph samples/graph/steps_wrong_order.py --format text
samples/graph/steps_wrong_order.py:32: report: 3 steps, 3 columns
samples/graph/steps_wrong_order.py:11:   (source columns) -> margin  [1 column]
samples/graph/steps_wrong_order.py:18:   (source columns) -> costs  [1 column]
samples/graph/steps_wrong_order.py:25:   (source columns) -> costs_again  [1 column]
samples/graph/steps_wrong_order.py:32:   reads from its source: columns=["quantity","revenue","shipping","unit_cost"]
samples/graph/steps_wrong_order.py:11: error: margin() reads the column total_cost, which costs() adds later in report(). The read raises KeyError. Move margin() after costs().
samples/graph/steps_wrong_order.py:27: error: costs_again() joins the column total_cost, which costs() added already. The join raises "columns overlap".
xpd: 1 pipeline with 3 steps in 1 file, 2 errors
```

A Hamilton module prints one line for each node, with the nodes it reads, and the names that no
function defines, which the caller has to pass.

```console
$ xpd graph samples/graph/features.py --format text
samples/graph/features.py:10: (no input) -> uk
samples/graph/features.py:14: (no input) -> dk
samples/graph/features.py:18: uk + dk -> spread
samples/graph/features.py:22: spraed -> spread_sq
inputs: spraed
read by no node: spread, spread_sq
samples/graph/features.py:22: warning: spread_sq reads spraed, which no function defines and which is one edit from the node spread. If it is a typing error, Hamilton raises "Required input spraed not provided" at execute.
xpd: 4 nodes from 4 functions in 1 file, 1 input, 1 warning
```

## Findings

| Finding | Severity | What happens |
| --- | --- | --- |
| A step reads a column that a later step adds | error | The read raises `KeyError` |
| A step joins a column that an earlier step added | error | The join raises "columns overlap" |
| A function name is defined twice in one Hamilton file, or two functions define one node | error | Python keeps the last `def`, or the Driver raises "Cannot define function ... more than once" |
| A Hamilton function has no type hint on its return value or a parameter | error | The Driver raises "Missing type hint" when it is built |
| A step computes the formula of a column that another step adds, or two columns have the same formula | warning | The same work is done twice, and the copies can come to differ |
| A step writes to the frame it was given, as `frame["x"] = ...` | warning | Called as `step(frame)`, it changes the caller's frame |
| The tree in the module docstring differs from the pipeline | warning | The description is out of date |
| A Hamilton input is one edit from a node name | warning | If it is a typing error, execute raises "Required input ... not provided" |
| `graph` cannot read a decorator | note | Nothing, but the graph may lack nodes |

## JSON output

```console
$ xpd graph samples/graph/steps.py --format json | jq -c 'keys, (.pipelines[0] | keys), (.pipelines[0].steps[0] | keys)'
["inputs","nodes","pipelines","problems","unread"]
["file","free","line","links","name","source","source_complete","steps"]
["columns","computed","kind","line","name","reads"]
```

`source` holds the columns that the pipeline reads from the frame it starts from, and
`source_complete` is false when a step also reads columns whose names the file computes.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | No error |
| 1 | At least one error |
| 2 | xpd could not run, or it could not read a file. It names that file on stderr and still reports the others |

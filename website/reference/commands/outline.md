Use `xpd outline` to map what files import and define before you read them in full. It lists
definitions, imports and members with line numbers and signatures, without running any code, in
any language that ast-grep parses.

## Examples

A file gets its definitions, each with the names of its members.

```console
$ xpd outline samples/graph/steps.py --format text
samples/graph/steps.py
14: LARGE_ORDER = 1000.0
15: PRICE = "unit_price"
18: def totals(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
28: def shares(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
33: def discounts(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
40: def flags(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
50: def features(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
```

A folder gets a shorter view, with the public names of each file grouped by kind. `--type`,
`--match` and `--view` narrow it down.

```console
$ xpd outline samples/graph --type function --match '^(totals|flags)' --view signatures --format text
samples/graph/impact.py
6: def totals(orders: pd.DataFrame) -> pd.DataFrame

samples/graph/steps.py
18: def totals(orders: pd.DataFrame) -> pd.DataFrame
40: def flags(orders: pd.DataFrame) -> pd.DataFrame
```

`--globs` keeps or drops paths by a glob, and `--follow` follows symbolic links when it walks
a folder.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The outline was printed |
| 2 | xpd could not run, or it could not read a path |

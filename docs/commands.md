# Commands

Run `xpd check` to lint Python code, and use the other commands below to search code, read
a dataflow, test a refactor and look up library APIs. `xpd -h` and `xpd <command> -h` print the
flags of the xpd you have installed.

| Command | What it does | Needs the project's Python |
| --- | --- | --- |
| [`check`](#check), also `scan` | Lints Python files and writes fixes | No |
| [`rules`](#rules) | Lists every rule with its severity, fix and documentation | No |
| [`run`](#run), also `query` | Finds code by syntax in any language that ast-grep parses, and rewrites it | No |
| [`outline`](#outline) | Lists the imports, definitions and members of files | No |
| [`graph`](#graph) | Prints the dataflow of step code and Hamilton modules | No |
| [`profile`](#profile) | Profiles a data file | Yes, with pandas |
| [`equiv`](#equiv) | Runs two scripts and compares their `result` | Yes, with pandas |
| [`refit`](#refit) | Refits a saved scikit-learn Pipeline and compares its outputs and scores | Yes, with pandas and scikit-learn |
| [`find`](#find) | Searches library symbols, rules and recipes for a task | Only once, to download the search model |
| [`api`](#api) | Prints one symbol, method, rule or recipe in full | No |
| [`index`](#index) | Builds a search index from library source | Yes, with Griffe 2.4.1 |
| [`new`](#new) | Creates a rule project, a rule, a test case or a utility | No |
| [`test`](#test) | Checks the test cases and snapshots of project rules | No |
| [`hook`](#hook) | Lints the files that a coding agent edited | No |
| [`lsp`](#lsp) | Starts the language server for an editor | No |
| [`mcp`](#mcp) | Starts the MCP server | Yes, with xpd installed as a Python package |
| [`skill`](#skill) | Prints the agent skill | No |
| [`completions`](#completions) | Prints shell completion | No |

The project's Python is the one in `$VIRTUAL_ENV`, or else in the nearest `.venv` at or above the
current folder, or else `python3` on `PATH`.

## check

`xpd check` lints the files and folders you name, or the current folder when you name none, and
prints one line for each issue.

```console
$ xpd check samples/bad.py --unsafe-fixes
samples/bad.py:5:15: warning [pandas/row-loop] iterrows() builds a Series for every row. Compute on whole columns instead, such as df["a"] * df["w"]. When each row has to become a Python object, loop over itertuples() or to_dict("records").
samples/bad.py:6:5: error [pandas/chained-assignment] [*] Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, "a"] instead.
samples/bad.py:7:1: info [pandas/no-inplace] [*] inplace=True gives no speedup and blocks chaining. Reassign instead.
samples/bad.py:8:5: warning [pandas/merge-without-validate] merge without validate= can silently duplicate rows.
samples/bad.py:11:5: warning [pandas/concat-in-loop] out grows by one concat per pass, and each concat copies every earlier row. Collect the frames in a list and concat once after the loop.
xpd: 5 issues, 2 fixable with --fix --unsafe-fixes
```

Each line gives the place, the severity, the rule and the message. `[*]` marks an issue that
`--fix` fixes with the flags you passed. Folders respect `.gitignore`.

| Rules | Run on |
| --- | --- |
| The rules of a library, such as `pandas/row-loop` | Files that import the library |
| The Python rules | Every file |
| A rule for a newer library version than the project's venv holds | No file, because xpd skips the rule |
| A package check, such as `pandas/missing-pyarrow` | The project, once per run, when its venv has the library and lacks the package |

See [Rules](rules.md) for every rule.

### Fixes

`--fix` writes the fixes and then lints what is left. A safe fix keeps what the code does. An
unsafe fix may change it, for example because another name still points at the old object.

| Flags | Effect |
| --- | --- |
| none | Lint. `[*]` marks an issue with a safe fix |
| `--fix` | Write the safe fixes, then lint what is left |
| `--unsafe-fixes` | Lint. `[*]` also marks an issue with an unsafe fix |
| `--fix --unsafe-fixes` | Write every fix, then lint what is left |
| `--fix --review` | Show each safe fix and ask before writing it. `-i` is short for `--review` |
| `--fix --review --unsafe-fixes` | Show each safe and unsafe fix and ask before writing it |

### Output and exit codes

The output is text on a terminal and JSON otherwise, and `--format` picks one of four formats.

| `--format` | What it prints |
| --- | --- |
| `text` | One line per issue, then a summary line |
| `json` | `fixed`, `fixable` and `issues`, as below |
| `sarif` | SARIF 2.1.0, with an entry for each rule, with its id, message and link, and a result for each issue, with its level, message and place |
| `github` | One GitHub Actions workflow command per issue, such as `::error file=samples/bad.py,line=6,col=5,title=pandas/chained-assignment::...` |

```console
$ xpd check samples/bad.py --format json | jq '{fixed, fixable, issue: .issues[1]}'
{
  "fixed": 0,
  "fixable": 0,
  "issue": {
    "column": 5,
    "file": "samples/bad.py",
    "fix": "unsafe",
    "line": 6,
    "message": "Chained assignment writes to a copy under copy-on-write, so the frame keeps its old values. Assign to df.loc[i, \"a\"] instead.",
    "package": "pandas",
    "rule": "chained-assignment",
    "severity": "error"
  }
}
```

`fixed` counts the fixes that `--fix` wrote, and `fixable` counts the issues that `--fix` would fix
with the same `--unsafe-fixes` setting. `fix` is `safe`, `unsafe` or `null`. A package check is
about the project and not about a place in a file, so its `line` and `column` are `null`.

| Exit code | Meaning |
| --- | --- |
| 0 | No issue of severity error is left, after fixes |
| 1 | At least one issue of severity error is left |
| 2 | xpd could not run, for example because a path cannot be read |

The format never changes the exit code or the rules that run.

### Select rules

`--rule` and `--severity` limit the run to some rules, and `rules` takes the same two flags.

| Flag | Effect |
| --- | --- |
| `--rule pandas` | Runs the rules of one package |
| `--rule row-loop` | Runs one rule, named without its package |
| `--rule pandas/chained-assignment` | Runs one rule, named with its package. Repeat `--rule` to name several |
| `--severity warning` | Runs the rules of that severity and above. The order is `error`, `warning`, `info`, `hint` |

### Project rules

A project adds its own rules in the nearest `sgconfig.yml` or `sgconfig.yaml` at or above the
current folder, and `--config` (`-c`) names another file.

| File or key | What it holds |
| --- | --- |
| `ruleDirs` in `sgconfig.yml` | Plain ast-grep rules, for any language |
| `utilDirs` in `sgconfig.yml` | Utilities that rules use with `matches: <name>` |
| `xpd.yml` beside `sgconfig.yml` | Library packs, each with a Python module and its own `ruleDirs` |

[`xpd new project`](#new) writes both files. A rule in a library pack gets the import gate,
version bounds and value proofs of a bundled pack, and a plain rule runs without them. A project
rule whose fix has no `metadata.fix` gets an unsafe fix.

xpd finds the language of each file by its file name. It warns when `sgconfig.yml` sets
`languageGlobs` or `languageInjections`, which it does not read, and it skips the rules and
utilities of `customLanguages`.

### Suppress an issue

A comment `# ast-grep-ignore: pandas/row-loop` turns the named rule off for the next line, or for
its own line when it follows code.

```python
# ast-grep-ignore: pandas/row-loop
for i, row in df.iterrows():
    pass
```

A comment without a rule name turns off every rule. Suppressions use public rule names, so one
name covers every variant of a rule. xpd reports a comment that suppresses nothing as
`xpd/unused-suppression`, with severity hint. A run with `--rule` or `--severity` skips the check for
unused suppressions, because a rule it left out could use the comment.

## rules

`xpd rules` lists every rule and package check of every pack, whatever the project's venv holds.

```console
$ xpd rules --format text | grep '(since' | head -3
info [arch/arch-unseeded-draws] (since 5.0) $C draws its samples from a new seed on every run, so the intervals and variances change from run to run, and np.random.seed does not reach it. Pass seed.
warning [arch/opt-bandwidth-as-bandwidth] (since 9.0) Since arch 9.0 $K uses opt_bandwidth minus 1 as its bandwidth, so passing opt_bandwidth back gives another estimate. Bartlett gave a long run variance of 0.5245 on its own and 0.5158 with bandwidth=opt_bandwidth. Pass the bandwidth attribute to repeat the estimate.
info [arch/reindex-is-default] (safe fix) (since 6.2) reindex=False is the default of forecast since arch 6.2, so this argument does nothing. Drop it.
```

Each text line gives the severity, the rule, its fix, the first library version it applies to, and
the message. The output is text on a terminal and JSON otherwise.

| Flag | Effect |
| --- | --- |
| `--builtin` | Lists only the rules that ship with xpd, without project rules |
| `--config FILE` | Reads project rules from another `sgconfig.yml` |
| `--rule NAME`, `--severity LEVEL` | Select rules, as in [`check`](#select-rules) |
| `--format text` or `json` | Picks the output format |

The JSON is a list with one object per rule.

| Field | Value |
| --- | --- |
| `package`, `rule`, `severity`, `message` | As in `check` |
| `fix` | `safe`, `unsafe` or `null` |
| `since` | The first library version the rule applies to, or `null` |
| `url` | The rule's section on [Rules](rules.md) |
| `doc` | The rule's section as Markdown |

A rule file may hold several documents, one for each form that the rule matches. `rules` lists
them as one rule, with the highest severity of the documents, the message of the first and the
least safe fix. `since` is the lowest version, or `null` when one document applies to every
version.

## run

`xpd run` finds code that matches a syntax pattern or a node kind, without running the code, and
can rewrite each match.

```console
$ xpd run --lang python --pattern '$X.iterrows()' samples/bad.py
samples/bad.py
5│for i, row in df.iterrows():
```

A rewrite is a preview until you pass a flag that writes it.

```console
$ xpd query -p 'pd.merge($$$A)' -r 'df.merge(other, on="k")' samples/bad.py
samples/bad.py
@@ -4,7 +4,7 @@
5  5 │ for i, row in df.iterrows():
6  6 │     df["a"][i] = row["a"] * 2
7  7 │ df.dropna(inplace=True)
8    │-m = pd.merge(df, other, on="k")
   8 │+m = df.merge(other, on="k")
9  9 │ out = pd.DataFrame()
10 10│ for f in ["x.csv"]:
11 11│     out = pd.concat([out, pd.read_csv(f)])
```

`query` is another name for `run`. A rewrite has no xpd value proof and no safe or unsafe class, so
read the preview before you write it.

| Flag | Effect |
| --- | --- |
| `--pattern`, `-p` | The code to match, with metavariables such as `$X` and `$$$ARGS` |
| `--kind`, `-k` | A syntax kind to match, or a selector such as `call > identifier` |
| `--selector KIND` | Makes the part of the pattern with that kind the match |
| `--lang`, `-l` | The language of the pattern. Without it, each file's extension names its language |
| `--rewrite`, `-r` | The code that replaces each match, with the metavariables filled in |
| `--interactive`, `-i` | Asks about each rewrite before writing it |
| `--update-all`, `-U` | Writes every rewrite |
| `--stdin` | Reads the code from stdin, in the language of `--lang` |
| `--format json` or `--json` | Prints the matches as JSON. JSON never writes files, so it cannot be combined with `-U` or `-i` |

`xpd run --help` lists the other flags, such as `--strictness`, `--globs` and `--context`.

```console
$ xpd run -p '$X.iterrows()' samples/bad.py --format json | jq -c '.[0] | keys, .range'
["charCount","file","language","lines","metaVariables","range","text"]
{"byteOffset":{"start":99,"end":112},"start":{"line":4,"column":14},"end":{"line":4,"column":27}}
```

Each JSON match has the file, the text, the byte range, the line and column range, the language
and the captured metavariables. Lines and columns in the JSON count from zero.

| Exit code | Meaning |
| --- | --- |
| 0 | Something matched |
| 1 | Nothing matched |
| 2 | xpd could not run |

## outline

`xpd outline` lists what files import and define, with line numbers and signatures, without running
any code.

```console
$ xpd outline samples/graph/steps.py
samples/graph/steps.py
14: LARGE_ORDER = 1000.0
15: PRICE = "unit_price"
18: def totals(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
28: def shares(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
33: def discounts(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
41: def flags(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
51: def features(orders: pd.DataFrame) -> pd.DataFrame
      variable: orders
```

A folder gets a shorter view, with the names of each file grouped by kind.

```console
$ xpd outline samples/graph
samples/graph/chain.py
constant: LIMIT
function: features, load
variable: margins

samples/graph/features.py
function: uk, dk, spread, spread_sq

samples/graph/steps.py
constant: LARGE_ORDER, PRICE
function: totals, shares, discounts, flags, features

samples/graph/steps_wrong_order.py
function: margin, costs, costs_again, report
```

| Flag | Effect |
| --- | --- |
| `--items` | `structure` lists definitions, `imports` lists imports, `exports` lists public names, and `all` lists everything. The default is `structure` for files and `exports` for folders |
| `--view` | `names`, `signatures`, `digest` or `expanded`. The default is `digest` for files and `names` for folders |
| `--type function,class` | Keeps the items of the named symbol types |
| `--match REGEX` | Keeps the items whose name or signature matches |
| `--pub-members` | Lists only public members |
| `--outline-rules FILE` | Adds outline rules in the schema of `ast-grep outline --outline-rules` |
| `--no-default-outline-rules` | Uses only the rules of `--outline-rules` |
| `--format json` or `--json` | Prints JSON, with each item's range, signature, kind and members |
| `--stdin` | Reads the code from stdin, in the language of `--lang` |

```console
$ xpd outline samples/graph --type function --match '^(totals|flags)' --view signatures
samples/graph/steps.py
18: def totals(orders: pd.DataFrame) -> pd.DataFrame
41: def flags(orders: pd.DataFrame) -> pd.DataFrame
```

## graph

`xpd graph` prints the dataflow of Python files, which part of the code reads the result of which
other part, and what does not connect. It imports nothing and runs nothing, so it needs no Python.

```sh
xpd graph [PATHS]... [--format text|json]
```

It reads a file in one of two ways.

| A file with | Is read as |
| --- | --- |
| A function that passes a frame through other functions of the file | [Step code](#step-code) |
| Anything else | [Hamilton modules](#hamilton-modules) |

A file with a pipeline that also imports `hamilton` is read both ways. So is a file whose only
pipelines are [method chains](#method-chains).

### Step code

`graph` lists the steps of each pipeline, the columns each step adds, and the steps it reads from,
and it reports a column that would be read before it exists. A step is a function that takes a
frame and returns it with columns added. A pipeline is a function that runs steps in order, as
`frame.pipe(a).pipe(b)` or as `frame = a(frame)` followed by `frame = b(frame)`.

```console
$ xpd graph samples/graph/steps.py
samples/graph/steps.py:51: features: 4 steps, 7 columns
samples/graph/steps.py:18:   (source columns) -> totals  [2 columns]
samples/graph/steps.py:28:   totals -> shares  [1 column]
samples/graph/steps.py:33:   (source columns) -> discounts  [1 column]
samples/graph/steps.py:41:   totals + discounts -> flags  [3 columns]
samples/graph/steps.py:51:   reads from its source: columns=["discount","quantity","unit_price"]
samples/graph/steps.py:35: warning: discounts() computes before_discount with the formula of the column gross_amount from totals(). Read the column gross_amount there.
samples/graph/steps.py:37: warning: discounts() writes to the frame it was given. Called as discounts(orders), it changes the caller's frame too. Return orders.assign(...) or write to orders.copy().
samples/graph/steps.py:45: warning: The column over_limit from flags() has the same formula as is_large from flags(), so the result holds one column twice. Keep one of them.
samples/graph/steps.py:51: warning: The tree in the module docstring lists totals, shares, flags and features() runs totals, shares, discounts, flags. Change the one that is out of date.
xpd: 1 pipeline with 4 steps in 1 file, 4 warnings
```

| Line | Meaning |
| --- | --- |
| `features: 4 steps, 7 columns` | The pipeline, named after its function |
| `totals + discounts -> flags  [3 columns]` | The step `flags` reads columns that `totals` and `discounts` add, and adds 3 columns |
| `(source columns) -> totals` | The step reads only columns of the frame the pipeline starts from |
| `reads from its source: columns=[...]` | The columns that the frame the pipeline starts from must have, written as the `columns=` argument of `pd.read_parquet` |
| `... and computed names` | Added after the list when a step also reads columns whose names the file computes |
| `order is free for: ...` | The steps that share no column with another step |

A step in the wrong order is an error, and the run exits 1.

```console
$ xpd graph samples/graph/steps_wrong_order.py
samples/graph/steps_wrong_order.py:32: report: 3 steps, 3 columns
samples/graph/steps_wrong_order.py:11:   (source columns) -> margin  [1 column]
samples/graph/steps_wrong_order.py:18:   (source columns) -> costs  [1 column]
samples/graph/steps_wrong_order.py:25:   (source columns) -> costs_again  [1 column]
samples/graph/steps_wrong_order.py:32:   reads from its source: columns=["quantity","revenue","shipping","unit_cost"]
samples/graph/steps_wrong_order.py:11: error: margin() reads the column total_cost, which costs() adds later in report(). The read raises KeyError. Move margin() after costs().
samples/graph/steps_wrong_order.py:27: error: costs_again() joins the column total_cost, which costs() added already. The join raises "columns overlap".
xpd: 1 pipeline with 3 steps in 1 file, 2 errors
```

| Finding | Severity | What happens |
| --- | --- | --- |
| A step reads a column that a later step joins | error | The read raises `KeyError` |
| A step joins a column that an earlier step added | error | The join raises "columns overlap" |
| A step computes the formula of a column that another step adds | warning | The same work is done twice, and the two copies can come to differ |
| Two columns have the same formula | warning | The result holds one column twice |
| Two steps write out the same formula | warning | As above, with no column to read it from |
| A column is assigned twice in one step with the same formula | warning | The first assignment is wasted |
| A step writes to the frame it was given, as `frame["x"] = ...` | warning | Called as `step(frame)`, it changes the caller's frame |
| An `.assign(...)` of a chain reads a column from the name the chain starts at, as `orders["net"]`, and an earlier `.assign(...)` of the chain adds that column | warning | The name holds the frame from before the chain, so the read raises `KeyError` or reads old values |
| The tree in the module docstring differs from the pipeline | warning | The description is out of date |
| A frame of new columns is not joined to the result | note | The result lacks the new columns |

Two formulas are the same when they are equal after each local name is replaced by what it stands
for and the terms of each sum and product are sorted. So `a + b` in one step equals `b + a` in
another.

| `graph` reads | As in |
| --- | --- |
| A column read with a literal name, a module constant, or a list of literal names and constants | `frame["price"]`, `frame[PRICE]` |
| A column read as an attribute, when another step adds a column of that name | `frame.total` |
| A column read with `pd.col`, or in a lambda that `.assign(...)` calls with the frame | `pd.col("price")`, `frame.assign(total=lambda d: d["price"] * d["units"])` |
| A column added by `.assign`, by an item assignment, or by a join of new columns | `frame.assign(total=...)`, `frame["total"] = ...`, `frame.join(pd.DataFrame(index=frame.index).assign(total=...))` |
| A step defined at module level | `def totals(orders): ...`. A method of a class is not read |

A step whose column names the file computes, as in `frame[prefix + "_total"]` or a join of
`pd.get_dummies(...)`, is marked `and computed names`. `graph` still reads its named columns, and
it leaves the step out of `order is free for`, because not all of its columns are known.

The docstring check applies to a file with one pipeline of functions whose module docstring draws
its steps as `├── step()` lines.

#### Method chains

`graph` reads a method chain as a pipeline too, with each `.assign(...)` of a chain of two or more
steps as one step, named by the columns it adds. The chain can stand in a function or in a
module statement, and it can mix `.assign(...)` with `.pipe(step)`. Formulas compare across the
three ways to read a column, so `pd.col("a") * 2` in one step equals `d["a"] * 2` in the lambda of
another.

```console
$ xpd graph samples/graph/chain.py
samples/graph/chain.py:12: features: 4 steps, 5 columns
samples/graph/chain.py:14:   (source columns) -> .assign(gross_amount)  [1 column]
samples/graph/chain.py:15:   .assign(gross_amount) -> .assign(net_amount, before_discount)  [2 columns]
samples/graph/chain.py:19:   .assign(net_amount, before_discount) -> .assign(customer_share)  [1 column]
samples/graph/chain.py:20:   .assign(net_amount, before_discount) -> .assign(is_large)  [1 column]
samples/graph/chain.py:12:   reads from its source: columns=["discount","quantity","unit_price"]
samples/graph/chain.py:28: margins: 2 steps, 2 columns
samples/graph/chain.py:30:   (source columns) -> .assign(unit_margin)  [1 column]
samples/graph/chain.py:31:   .assign(unit_margin) -> .assign(margin)  [1 column]
samples/graph/chain.py:28:   reads from its source: columns=["quantity","unit_cost","unit_price"]
samples/graph/chain.py:12: orders -> features
samples/graph/chain.py:24: path -> load
inputs: orders, path
read by no node: features, load
samples/graph/chain.py:17: warning: The column before_discount from .assign(net_amount, before_discount) has the same formula as gross_amount from .assign(gross_amount), so the result holds one column twice. Keep one of them.
samples/graph/chain.py:19: warning: .assign(customer_share) reads orders["net_amount"], and orders is the frame from before the chain, without the column net_amount that .assign(net_amount, before_discount) adds. The read raises KeyError, or reads old values when orders has such a column. Read it as pd.col("net_amount").
xpd: 2 pipelines with 6 steps in 1 file, 2 nodes from 2 functions, 2 inputs, 2 warnings
```

A pipeline in a module statement is named after the variable it is assigned to, as `margins` is
above. A function that another pipeline runs as a step counts as one step, and its own chain
is not listed as a second pipeline. The file has only chains as pipelines, so its functions are
also listed as [Hamilton nodes](#hamilton-modules), in the lines from `orders -> features` to
`read by no node`.

### Hamilton modules

`graph` prints which node of [Apache Hamilton](https://hamilton.apache.org/) modules reads which,
and it reports names that would make the Driver fail. Hamilton turns each function of a module
into a node, with the function name as the output and the parameter names as the inputs. `graph`
needs no Python and no Hamilton.

Pass the modules that you give to one Driver, as in `with_modules(loaders, features)`, because
together they make one dataflow. A folder means every `.py` file in it. Leave out the script that
builds the Driver, or its functions show as nodes.

```console
$ xpd graph samples/graph/features.py
samples/graph/features.py:10: (no input) -> uk
samples/graph/features.py:14: (no input) -> dk
samples/graph/features.py:18: uk + dk -> spread
samples/graph/features.py:22: spraed -> spread_sq
inputs: spraed
read by no node: spread, spread_sq
samples/graph/features.py:22: warning: spread_sq reads spraed, which no function defines and which is one edit from the node spread. If it is a typing error, Hamilton raises "Required input spraed not provided" at execute.
xpd: 4 nodes from 4 functions in 1 file, 1 input, 1 warning
```

| Line | Meaning |
| --- | --- |
| `uk + dk -> spread` | The node `spread` reads the nodes `uk` and `dk`. An input with `?` has a default |
| `inputs: spraed` | Names that no function defines, which the caller has to pass |
| `read by no node: ...` | The ends of the dataflow |

| Finding | Severity | What happens in Hamilton |
| --- | --- | --- |
| A function name is defined twice in one file | error | Python keeps the last `def`, so the others never become nodes. Two `@config.when` versions need the names `rate__uk` and `rate__dk` |
| Two functions define one node | error | The Driver raises "Cannot define function ... more than once" when it is built |
| A plain `def rate` stands beside `@config.when` on `rate__dk` | error | The same error, once the config selects the version |
| The return value or a parameter has no type hint | error | The Driver raises "Missing type hint" when it is built |
| An input is one edit from a node name | warning | If it is a typing error, execute raises "Required input ... not provided" |
| `graph` cannot read a decorator | note | Nothing, but the graph may lack nodes |

| Decorators | What `graph` does |
| --- | --- |
| `@config.when` and its relatives, `@extract_columns`, `@extract_fields`, `@unpack_fields`, `@parameterize`, `@parameterize_values`, `@parameterize_sources`, `@inject`, `@load_from`, `@save_to` | Reads them when their arguments are literals |
| A decorator whose names the file computes, such as `@extract_columns(*COLUMNS)` | Prints a note |
| `@subdag`, `@pipe_input`, `@pipe_output`, `@mutate`, `@with_columns`, `@does`, `@resolve` | Prints a note |

`graph` does not know the config that the Driver gets, so it lists every version of a node. It
does not compare type hints between nodes.

### Output and exit code

The output is text on a terminal and JSON otherwise.

```console
$ xpd graph samples/graph/steps.py --format json | jq -c 'keys, (.pipelines[0] | keys), (.pipelines[0].steps[0] | keys)'
["inputs","nodes","pipelines","problems","unread"]
["file","free","line","links","name","source","source_complete","steps"]
["columns","computed","kind","line","name","reads"]
```

| JSON key | Each item has |
| --- | --- |
| `pipelines` | `name`, `file`, `line`, `steps`, `links`, `source`, `source_complete` and `free` |
| `pipelines[].steps` | `name`, `kind`, `line`, `reads`, `columns` and `computed`. `kind` is `function`, or `link` for one `.assign(...)` of a chain |
| `pipelines[].links` | `from`, `to` and `columns` |
| `nodes` | `name`, `file`, `line`, `function`, `inputs`, `kind` and `config` |
| `inputs` | `name`, `optional` and `readers` |
| `unread` | A node name |
| `problems` | `file`, `line`, `severity` and `message` |

`source` holds the columns that the pipeline reads from the frame it starts from, and
`source_complete` is false when a step also reads columns that the file does not name.

| Exit code | Meaning |
| --- | --- |
| 0 | No error |
| 1 | At least one error |
| 2 | xpd could not run, for example because a path cannot be read |

## profile

`xpd profile` prints JSON with the dtype, null count, distinct count and memory use of each column
of a data file, and findings about how to load it.

```console
$ xpd profile samples/data.csv
{"file": "samples/data.csv", "pandas": "3.1.1", "rows": 3, "memory_bytes": 306, "columns": [{"column": "a", "dtype": "float64", "nulls": 1, "unique": 2, "memory_bytes": 24}, {"column": "b", "dtype": "str", "nulls": 0, "unique": 2, "memory_bytes": 150}], "findings": []}
```

It reads `.csv`, `.parquet`, `.feather`, `.json` and `.xlsx` files, and it runs with the project's
Python, which needs pandas. Each finding has `finding`, `columns` and a `message` with what was
measured on the file.

| Finding | When | What the message names |
| --- | --- | --- |
| `values-as-column-names` | Three or more column names are years, are dates, or share a stem and differ in a number, as `sales_2022`, `sales_2023` and `sales_2024` do | The `melt` call that gives one row per value, with the other columns as `id_vars` |
| `category-candidate` | A text column has at most half as many distinct values as rows, and takes at most half the memory as a category | The bytes it takes as a category and as it is, and `dtype={"city": "category"}` |

```console
$ xpd profile samples/pipeline/penguins.csv | jq -r '.findings[].message'
species holds 3 distinct texts in 344 rows. As a category it took 512 bytes, where it takes 19,124 as str. Pass dtype={"species": "category"} to the reader, or call astype("category").
island holds 3 distinct texts in 344 rows. As a category it took 511 bytes, where it takes 18,952 as str. Pass dtype={"island": "category"} to the reader, or call astype("category").
sex holds 2 distinct texts in 344 rows. As a category it took 452 bytes, where it takes 18,331 as str. Pass dtype={"sex": "category"} to the reader, or call astype("category").
```

## refit

`xpd refit` loads a saved scikit-learn Pipeline, fits a clone of it on the raw training rows, and
tells you whether the outputs and scores match the ones its training script saved. Run it in the
project root, with pandas and scikit-learn in the project's Python. The training script in
`samples/pipeline` writes a model and a contract that you can try it on.

```sh
cd samples/pipeline
python train.py
xpd refit model/contract.json
```

```console
$ xpd refit model/contract.json
{"rows": 344, "train_rows": 258, "test_rows": 86, "output": "predict_proba", "largest_gap": 0.0, "model_matches": true, "contract_test_score": 0.9883720930232558, "refit_test_score": 0.9883720930232558, "contract_cv_scores": [0.9807692307692307, 0.9807692307692307, 1.0, 1.0, 0.9803921568627451], "refit_cv_scores": [0.9807692307692307, 0.9807692307692307, 1.0, 1.0, 0.9803921568627451], "test_score_matches": true, "cv_scores_match": true, "counts_match": true, "passed": true}
```

`refit` takes the steps below in order.

1. First, it reads the data file and rebuilds the split that the contract records.
2. Second, it loads the model with joblib and fits a clone on the training rows.
3. Third, it compares the outputs of the model and the clone on the held out rows. It uses
   `predict_proba` when the model has it, then `decision_function`, then `predict`.
4. Fourth, it scores the held out rows with `get_scorer`, and it computes fold scores with
   `cross_val_score` on the training rows only.

The contract must record `validation.splitter`, with `class` naming a splitter in
`sklearn.model_selection` and `args` holding its keyword arguments, as
`{"class": "StratifiedKFold", "args": {"n_splits": 5, "shuffle": true, "random_state": 0}}`. Every
path in the contract must be inside the current folder. `refit` changes no saved file.

`largest_gap` is null for a prediction that is not numeric, and `refit` still compares it.
DataFrames and Series are compared as in [`equiv`](#equiv), with the pandas default relative
tolerance of 0.00001 and absolute tolerance of 0.00000001 for floats. Other outputs and the
scores must be exactly equal under `numpy.testing.assert_equal`, so save the contract scores
without rounding.

| Exit code | Meaning |
| --- | --- |
| 0 | Every comparison matches |
| 1 | A comparison differs |
| 2 | `refit` could not run, and the JSON has an `error` |

> [!WARNING]
> joblib runs code when it loads a model, so load only a model you trust.

## equiv

`xpd equiv` runs two scripts and tells you whether the variable `result` that each one leaves is
equal, so you can check a refactor of data code.

```console
$ xpd equiv samples/equiv/old.py samples/equiv/new.py
{"equivalent": true}
$ xpd equiv samples/equiv/old.py samples/equiv/wrong.py
{"equivalent": false, "detail": "DataFrame.iloc[:, 2] (column name=\"w\") are different\n\nDataFrame.iloc[:, 2] (column name=\"w\") values are different (100.0 %)\n[index]: [0, 1, 2]\n[left]:  [2, 4, 6]\n[right]: [3, 6, 9]"}
```

DataFrames and Series are compared with `pandas.testing`, and anything else with
`numpy.testing`, so a list and an array with the same values count as equal.

Each script runs in its own Python process, in the folder where you run `xpd equiv`. The second
script gets no module, no `sys.path` entry and no variable from the first. What a script prints
goes to stderr, so stdout holds only the JSON. Each `result` comes back as a pickle, so it cannot
be an object of a class that the script itself defines.

| Exit code | Meaning |
| --- | --- |
| 0 | `{"equivalent": true}` |
| 1 | The results differ, with the difference in `detail` |
| 2 | A script failed, exited or left no `result` that pickle can save, or pandas is missing. The JSON has an `error` |

## find

`xpd find` searches the library symbols, rules and worked recipes that ship with xpd for a task
you describe in words, and it needs no library installed.

```console
$ xpd find "rounded bars" --kind recipe
Recipes
plotly/recipe/rounded-bars [recipe]
Rounded Bars
pandas/recipe/round-each-value-in-the-index-to-the [recipe]
Round each value in the Index to the given number of decimals.
numpy/recipe/round-to-nearest-integer-towards-zero [recipe]
Round to nearest integer towards zero.
```

Without `--kind`, the text output has the headings `Symbols`, `Rules` and `Recipes`, each ranked on
its own. A symbol line shows the names of its parameters and its rule links, and a long signature
ends in `...` in text. JSON keeps every name, and [`api`](#api) prints each parameter as the source
writes it.

| Flag | Effect |
| --- | --- |
| `--kind symbol`, `rule` or `recipe` | Searches one group |
| `--limit N` | Returns up to N entries of each group. The default is 5 symbols, 3 rules and 3 recipes |
| `--package NAME` | Searches one library, by its Python import name, such as `sklego` |
| `--format json` | Prints `{"symbols": [...], "rules": [...], "recipes": [...]}`, with an empty list for a group that is left out or has no match. The default is text |
| `--index FILE` | Adds a local index from [`xpd index`](#index) |
| `--check FILE` | Runs the discovery questions of FILE, as in [Discovery questions](#discovery-questions) |

`find` exits 0 when nothing matches. Class members are not search results, so use `api` to read
them. A match means the entry fits the words of the query, and you still need to read the API and
its rules before you use it.

### Ranking

`find` ranks each group by keywords and by meaning, so "make the bars have soft corners" finds the
recipe for rounded bars. It answers "what is the thing called that does X", but it cannot answer
questions about the properties of a method, such as which linear models need no scaling.

| Part | How it ranks |
| --- | --- |
| Keywords | BM25 over the words of names, signatures, docstring summaries, rule texts and recipe titles. Names are split at underscores and camel case, and name words get weight three. Parameter names, source documentation and member summaries get lower weights. Queries and entries share one suffix stemming |
| Bonuses | Exact acronym names and phrases get a bonus, and shorter public paths rank higher |
| Meaning | The query is embedded with the EmbeddingGemma 300M model and compared with a stored vector of each entry. `indexes/vectors.bin` keeps the first 256 values of each vector |
| Combined | Within each group, reciprocal rank fusion combines the two rankings, and the top keyword matches get a bonus, so an exact match keeps its place |

The first `find` downloads the model once, 333 MB, and prints a line on stderr before it starts.
The download runs with the project's Python and writes under a temporary name, then moves the file
into place. Later searches run offline.

| System | Where the model is kept |
| --- | --- |
| `XDG_CACHE_HOME` is set | `$XDG_CACHE_HOME/xpd/models` |
| Linux | `~/.cache/xpd/models` |
| macOS | `~/Library/Caches/xpd/models` |
| Windows | `%LOCALAPPDATA%\xpd\models` |

`find` ranks by keywords only when there is no Python, the download fails or the model does not
load, and it prints one line on stderr that says why. A search with `--index` also ranks by
keywords only.

```console
$ XDG_CACHE_HOME=/nonexistent-cache-dir xpd find "rounded bars" --kind recipe
xpd: cannot create /nonexistent-cache-dir/xpd/models for the embedding model: Permission denied (os error 13). Search uses keywords only.
Recipes
plotly/recipe/rounded-bars [recipe]
Rounded Bars
pandas/recipe/round-each-value-in-the-index-to-the [recipe]
Round each value in the Index to the given number of decimals.
numpy/recipe/round-to-nearest-integer-towards-zero [recipe]
Round to nearest integer towards zero.
```

## api

`xpd api` prints one symbol, method, rule or recipe in full, with each parameter as the source
writes it, the first paragraph of the docstring, the public methods and the rules that may guard
it. A parameter keeps its type annotation and its default, so you can write the call without
importing the library. A stub file writes `...` in place of a default, so such a parameter is
optional and the text does not give its value.

```console
$ xpd api sklego.meta.GroupedPredictor
sklego.meta.GroupedPredictor(estimator, groups, shrinkage=None, use_global_model=True, check_X=True, shrinkage_kwargs=None) [class]
`GroupedPredictor` is a meta-estimator that fits a separate estimator for each group in the input data.
  decision_function(X) Predict confidence scores for samples in `X`.
  fit(X, y) Fit one estimator for each group of training data `X` and `y`.
  predict(X) Predict target values on new data `X` by predicting on each group. If a group is not found during `.predict()` and `use_global_model=True` the fallback estimator will be used. If `use_global_model=False` a `ValueError` will be raised.
  predict_proba(X) Predict probabilities on new data `X`.
Rules: sklego/grouped-shrinkage-arguments
```

The member lines of a class show the parameter names only, and a property has no parentheses. To
read one method or property on its own, add its name to the name of its class. An inherited member
works the same way.

```console
$ xpd api mapie.regression.TimeSeriesRegressor.update
mapie.regression.TimeSeriesRegressor.update(X: ArrayLike, y: ArrayLike, ensemble: bool = False, confidence_level: Optional[Union[float, Iterable[float]]] = None, gamma: float = 0.0, optimize_beta: bool = False) [method]
Update conformity scores
$ xpd api arch.univariate.base.ARCHModelResult.forecast
arch.univariate.base.ARCHModelResult.forecast(params: ArrayLike1D | None = None, horizon: int = 1, start: int | DateLike | None = None, align: ForecastAlignment = "origin", method: ForecastingMethod = "analytic", simulations: int = 1000, rng: Callable[[int | tuple[int, ...]], Float64Array] | None = None, random_state: np.random.RandomState | None = None, *, reindex: bool = False, x: dict[Label, ArrayLike] | ArrayLike | None = None) [method]
Construct forecasts from estimated model.
$ xpd api arch.unitroot.KPSS.pvalue
arch.unitroot.KPSS.pvalue [property]
The p-value for the test statistic.
```

`--format json` prints a list with the one entry. `parameters` holds the names, and
`parameters_written` holds the parameter list as the source writes it. An entry without parameters
has no `parameters_written`. `callable` is `class` or `function` for a symbol, `method` or
`property` for a member of a class, and `null` for a rule or a recipe.

```console
$ xpd api sklearn.model_selection.TimeSeriesSplit.split --format json
[{"kind":"symbol","package":"sklearn","name":"sklearn.model_selection.TimeSeriesSplit.split","callable":"method","parameters":["X","y","groups"],"parameters_written":["X","y=None","groups=None"],"summary":"Generate indices to split data into training and test set.","members":[],"rules":[]}]
```

`--index FILE` adds a local index, which must have the [index schema](#index-file) of this xpd. A
name that is not in the index exits 2.

| Entry | What `api` shows |
| --- | --- |
| A class | The parameters of its nearest `__init__`, or of its nearest `__new__` when it has no `__init__`, and its public methods and properties, with the inherited ones |
| A method or property of a class, as `Class.member` | The parameters of the method and the first paragraph of its docstring. A property has no parameter list. The entry has no rule links |
| The first parameter of a method or of a constructor, such as `self` or `cls` | Nothing, because Python binds it to the object or the class and a call does not pass it. A static method keeps its first parameter. A function of a module keeps a parameter named `self` or `cls`, such as the `cls` of `pandas.compat.set_function_name` |
| Any parameter | The text of the source or stub file, with its type annotation and its default |
| A parameter that the source writes over several lines | One line, without its comments and without a comma before a closing bracket. A line break inside a string is printed as `\n` |
| A default that the source writes as `...`, such as those of `pandas.Timestamp.replace` | `= ...`. A stub file writes it in place of the real default, so the parameter is optional and the library's documentation has the value. Many numpy, pandas and scipy stubs do so |
| A `*` or `/` that stands alone in the parameter list | The same character, so the parameters after `*` are keyword-only and those before `/` are positional-only. `parameters_written` holds it as an item of its own |
| A function that exists only as overloads in a stub file, such as `numpy.zeros` | The parameter names of all its overloads, without type annotations and defaults, because each overload writes them in its own way |
| The `__init__` that Python builds for a dataclass | Each field as Griffe reads it, with its annotation and default, because the source has no parameter list |
| A compiled function without a stub | No entry, because the index leaves it out |
| A NumPy universal function, such as `numpy.add` | No entry, because its stub declares it as an attribute |
| A local index with index schema 1, which an earlier xpd built | No entry, because xpd refuses the file until you build the index again |

The printed text says how to call the symbol. It does not say what a parameter means, so read the
library's documentation for that.

The rule links come from the words of the symbol's name in the pack's rule patterns, regex
patterns included. They show rules that may apply, and they do not prove that every use of the
symbol matches a rule. A rule entry holds the rule's full documentation, so a search for words such
as "small multiples" finds it. A rule prints as `plotly/bar-axis-without-zero [rule]`, without a
call signature or the metavariables of its pattern.

A recipe prints its title, its source pinned to a revision, the symbols it uses, its license name,
whether xpd fixes changed it, the copyright lines of its source file when it has any, and the code.

````console
$ xpd api numpy/recipe/returns-array-of-indices-of-the-maximum-values
numpy/recipe/returns-array-of-indices-of-the-maximum-values [recipe]
Returns array of indices of the maximum values along the given axis.
Source: https://github.com/numpy/numpy/blob/v2.5.5/numpy/ma/core.py#L5763
Symbols: numpy.arange
License: BSD-3-Clause
Modified by xpd fixes: false
Copyright 1999, 2000, 2001 Regents of the University of California.

```python
import numpy as np
a = np.arange(6).reshape(2,3)
a.argmax()
a.argmax(0)
a.argmax(1)
```
````

A recipe name is the pack, `recipe`, and the first eight words of its title, so it stays the same
when source paths or line numbers change. A pack keeps one recipe of each name. The full license
texts ship once, in the `licenses` folder of the package. In JSON, `recipe.modified` records
whether xpd applied fixes, and `recipe.code` is null for a source whose license xpd does not
recognize.

## index

`xpd index` builds a search index from the Python source and stub files of a library, so that
`find` and `api` can search a library or a version that xpd does not ship. It never imports or runs
library code.

```sh
uv run --with griffe==2.4.1 xpd index /path/to/site-packages --package sklego
uv run --with griffe==2.4.1 xpd index /path/to/source/sklego --package sklego --version 0.9.10 --output local.json
xpd find "grouped regression" --index local.json
xpd api sklego.meta.GroupedPredictor --index local.json
```

`xpd index` parses the source with Griffe 2.4.1 in the project's Python, so start it with
`uv run --with griffe==2.4.1`. No other command needs Griffe. Without it, the command stops
and prints the command to run.

```console
$ xpd index samples/sklego --package sklego --version 0.9.10
xpd: Index building needs griffe==2.4.1. Run uv run --with griffe==2.4.1 xpd index samples/sklego --package sklego --version 0.9.10.
```

| Flag | Effect |
| --- | --- |
| `--package NAME` | The Python import name of the library. Required |
| `--version V` | The library version. Without it, the version comes from the wheel metadata, so pass it for a source checkout |
| `--output FILE` | The file to write. The default is `indexes/<package>.json`, and the folder must exist |
| `--examples DIR` | The documentation and gallery folder to harvest recipes from. Needs `--source-url` and `--license` |
| `--source-url URL` | The repository URL, ending in `/blob/<revision>` with a tag or commit |
| `--license FILE` | The license file of the source, which names the license of each recipe |

Without `--examples`, the command keeps the recipes that the output file already holds for the
same package and version. A local index replaces the shipped index of its package, and it also
works for a library without an xpd rule pack.

### Index file

Each index file has `schema_version`, `package`, `version` and `entries`. `schema_version` is 2.
Schema 1 did not record which members of a class are properties, so `find` and `api` refuse a
local index that has it.

```console
$ xpd api arch.unitroot.ADF.pvalue --index old.json
xpd: old.json has index schema 1, and this xpd reads schema 2. Build the index again with xpd index.
```

| Key | What it holds |
| --- | --- |
| Every entry | `kind`, `package`, `name`, `callable`, `parameters`, `summary`, `members` and `rules`, and `keywords` taken from the source when there are any |
| An entry or a member with parameters | `parameters_written`, the parameter list as the source writes it. `parameters` holds the names |
| A member | `name`, `parameters` and `summary`, and `property`, which is `true` for a property and left out for a method |
| A recipe entry | `recipe`, with `title`, `symbols`, `source`, `code`, `license` and `modified`, and `attribution` with the copyright lines of the source file when it has any |
| `recipes` | The counts of a harvest, with `candidates`, `clean`, `fixed`, `dropped`, `capped` and `retained` |

### Public names

The index lists a name when it is public. A name is public when Griffe says it is. A module is
public when its name has no leading underscore. Another name is public when its module lists it in
`__all__`, or, in a module without `__all__`, when it has no leading underscore and the module
defines it. The index adds four rules, and each one comes from a named symbol.

| Rule | Symbol that needs it |
| --- | --- |
| Names in a folder named `tests` are left out | `pandas.tests.frame.test_api.TestDataFrameMisc` is in the wheel and is not library API |
| A class or function that a module defines under a public name is public, even when `__all__` leaves it out | `pandas.tseries.holiday.USFederalHolidayCalendar` |
| A name from `from module import *` is public, even when the `__all__` of the importing module leaves it out. The module that the import reads must give the name | `pandas.api.types.is_numeric_dtype` is public. `scipy.stats.interp1d` is not a name |
| An import into a module without a readable `__all__` is public, when the name is defined in that module's package or below it | `numpy.lib.stride_tricks.as_strided` is public. `pymoo.vendor.vendor_scipy.LocalSearch` is imported from another package and stays private |

Each definition gets one entry for each public name it has. When several public paths reach the
same name, the entry uses the path with the fewest parts, then the shortest text, then lexical
order. `numpy.concat` and `numpy.concatenate` are two names for one function, and both have an
entry.

Griffe drops some callables that a library has, and the index keeps them through a Griffe
extension and a Griffe loader subclass in `py/index.py`.

| Case | Symbol |
| --- | --- |
| A function that exists only as overloads in a stub file | `numpy.zeros` |
| A plain assignment to another callable | `numpy.concat`, `pandas.io.pytables.Term`, `numpy.int64` |
| A function that its stub declares as an attribute | `numpy.polynomial.hermite.hermadd` |
| A class or function with the same name as the submodule that defines it | `numpy.memmap`, `scipy.optimize.least_squares` |
| An `__all__` that is computed and cannot be read | `scipy.signal.butter` |
| An import beside a stand-in definition that takes `*args, **kwargs` | `plotly.figure_factory.create_hexbin_map` |

### Recipes

`--examples` harvests worked recipes from the documentation of a library checkout that is pinned
to a version. Unpack the library source into a new empty folder under `/var/tmp`, and run the index
command with all three recipe flags.

```sh
uv run --with griffe==2.4.1 xpd index /var/tmp/xpd-source/scikit-lego-0.9.10 --package sklego --version 0.9.10 \
  --examples /var/tmp/xpd-source/scikit-lego-0.9.10 \
  --source-url https://github.com/koaning/scikit-lego/blob/v0.9.10 \
  --license /var/tmp/xpd-source/scikit-lego-0.9.10/LICENSE
```

| Library | Folder for `--examples` |
| --- | --- |
| Plotly | The repository's `doc/python` folder, with `/doc/python` after the revision in `--source-url` |
| scikit-learn | The repository root, so the harvest reads `examples/` and the source docstrings |

The harvester reads the Examples sections of Python docstrings, Python files under examples or
gallery folders, Python fences in Markdown, RST doctests and notebook code cells. It never imports
or runs the code it reads.

| Step | What the harvester does |
| --- | --- |
| Reject | Snippets with syntax errors, placeholders, missing names, no imports, or no call into the library |
| Fix | Runs the rule engine, applies safe and unsafe fixes, and rejects a snippet with any issue left, including info issues |
| License | Names the license as `Apache-2.0`, `NCSA`, `MIT` or `BSD-3-Clause` from the `--license` file, and keeps the copyright lines of the source file. Another license gives a pointer to the source with no copied code |
| Select | Keeps up to 24 recipes and 96,000 code bytes per pack, and 8,000 bytes per snippet. It prefers short examples of public APIs with different primary symbols |

`capped` counts clean examples that the limits or a repeated primary symbol left out, and
`dropped` counts candidates rejected before selection. Copy the license and any NOTICE, DISCLAIMER
or third party notices into `licenses/`, which ships with xpd. Parsing and linting do not prove
that external datasets or optional dependencies are there. To refresh a pack, change the source
version and the pinned URL together, and run the command again to rebuild both symbols and
recipes.

### Discovery questions

`xpd find --check indexes/questions.json` asks the discovery questions in the file and exits 2 when
an expected entry is not in the top five of its own kind.

```console
$ xpd find --check indexes/questions.json
49 of 49 questions passed with keywords
53 of 53 questions passed with keywords and the embedding model
6 open questions are recorded and not checked
```

| Question field | Effect |
| --- | --- |
| `query` | The question, asked with keywords only and again with the model when the model is there |
| `expected` | One entry name, or a list of names that each answer the question. One of them in the top five passes |
| `"needs_model": true` | Asks the question with the model only |
| `"open": true` | Records what a user asked and the answer they expect, for later work on search. The check skips it |

The check prints every failure. It looks at the top five also for rules and recipes, whatever
the display defaults are, and it also exits 2 when an index entry has no vector of its current text. Lefthook runs the
check before a commit that changes `indexes/*.json` or a Rust file, and it tolerates no miss. A
question that still fails stays as it is, without a change to its wording and without a ranking
rule for that question.

After a change to `indexes/` or to a rule's text, write the vectors with
`xpd find --write-vectors indexes/vectors.bin`. The flag is hidden from `--help`. It embeds the
entries that are new or whose text changed, and it keeps the vector of every other entry.

## new

`xpd new` creates a rule project, or a rule, a test case or a utility in one, and asks for what is
missing when you name no item.

```sh
xpd new
xpd new project --pack team --yes
xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes
xpd new test team/no-print --yes
xpd new util print-call --lang python --pattern 'print($X)' --yes
```

The `project`, `rule` and `util` commands above, run in an empty folder, write the files below.
Each new folder also gets an empty `.gitkeep`.

```text
.
├── sgconfig.yml                       # ruleDirs: rules, utilDirs: utils, testConfigs: rule-tests
├── xpd.yml                            # the pack team, with ruleDirs: rules/team
├── docs/rules.md                      # the documentation section of team/no-print
├── rules/team/no-print.yml
├── rule-tests/team/no-print-test.yml  # id: team/no-print, valid: [], invalid: []
└── utils/print-call.yml
```

| Item | What it makes |
| --- | --- |
| `project` | `sgconfig.yml`, `xpd.yml`, and the `rules`, `rule-tests` and `utils` folders, with relative paths. Each `--pack MODULE` declares a pack whose rules run on files that import the module, or on every Python file with `--every-file` |
| `rule` | A rule, with its section in `docs/rules.md`. A rule in a pack is a Python rule named `<pack>/<name>`. A rule outside a pack is a plain rule for any language, set with `--language`, and has no import gate |
| `test` | An empty test case for a rule id or a public id. It writes no valid or invalid code |
| `util` | A utility rule, which any rule uses with `matches: <name>` |

| Flag | Effect |
| --- | --- |
| `--yes`, `-y` | Never asks, and fails when a needed flag is missing |
| `--directory DIR` | The project folder. The default is the nearest folder with `sgconfig.yml`, or the current folder for a new project |
| `--format json` | Prints `createdFiles` and `modifiedFiles` as absolute paths |

A rule needs a valid `--pattern` and a `--message`, and xpd checks it as `xpd check` and
`ast-grep scan` read it before it writes the file. `new` never overwrites a rule file, and it
exits 2 when the rule exists.

| Rule flag | Effect |
| --- | --- |
| `--fix CODE` with `--fix-safety safe` or `unsafe` | The code that replaces a match. `--fix` needs `--fix-safety` |
| `--severity` | `error`, `warning`, `info` or `hint`. The default is `warning` |
| `--since V`, `--until V` | The first library version the rule applies to, and the first it no longer applies to |
| `--holds VAR=RULE` | Keeps a match only when the variable holds a value of the rule's shape |
| `--unless-holds VAR=RULE` | Drops a match when the variable holds a value of the rule's shape |
| `--unless-imported RULE` | Turns the rule off in a file with a module level import statement of the rule's shape |
| `--public-id ID` | The name that issues show, so several variants of a rule share one name |
| `--note TEXT` | A longer explanation, for the rule's note and its documentation section |
| `--with-test` | Also creates an empty test case |

`xpd new help rule` prints the same help as `xpd new rule --help`.

## test

`xpd test` checks the valid and invalid examples of each project rule, and the snapshots of their
issues and fixes. In the project that [`xpd new`](#new) makes, with the test file below, the first
run fails because no snapshot exists yet, and `-U` writes the snapshot.

```yaml
id: team/no-print
valid:
  - "log(1)"
invalid:
  - "import team\nprint(1)"
```

```console
$ xpd test
team/no-print: passed
team/no-print: snapshot mismatch
2 cases, 1 failed
$ xpd test -U
team/no-print: passed
team/no-print: updated
2 cases, 0 failed
```

A test file has the ast-grep fields `id`, `valid` and `invalid`. A valid example must give no
issue from its rule, and an invalid example must give one. A case can set `path`, which rule path
filters and language detection read. The default engine runs each example through the same
scanner as `check`, with import gates, value proofs and version selection.

| Flag | Effect |
| --- | --- |
| `--filter REGEX`, `-f` | Tests the rules whose id matches |
| `--skip-snapshot-tests` | Checks only valid and invalid |
| `--update-all`, `-U` | Accepts every changed snapshot |
| `--interactive`, `-i` | Asks about each changed snapshot |
| `--test-dir DIR`, `-t` | Reads the tests of DIR instead of the `testConfigs` of `sgconfig.yml` |
| `--snapshot-dir DIR` | The snapshot folder inside the test folder. The default is `__snapshots__` |
| `--config FILE`, `-c` | Reads another `sgconfig.yml` |
| `--format json` | Prints each case with its `id`, `source` and `status`, and the `failed` count |
| `--engine structural` | Runs ast-grep's own test runner, suite format and snapshots |

A snapshot records the ranges of the issues and the proposed fixes, in a file such as
`rule-tests/__snapshots__/team-no-print-snapshot.yml`. The structural engine checks plain rules
without the import gates or value proofs of xpd, so use the default engine for a rule whose
examples depend on them.

| Exit code | Meaning |
| --- | --- |
| 0 | Every example passed |
| 1 | An example or a snapshot failed |
| 2 | The command could not run, for example because no case matched |

## hook

`xpd hook` lints the `.py` files that a coding agent changed, when it runs as a `PostToolUse` hook
and reads the hook event on stdin. See [Claude Code](claude-code.md) for the setup.

| Client | How the hook answers |
| --- | --- |
| A client of the Agent Plugins 1.1.0 draft | Prints the issues as `additional_context` in JSON, and exits 0 |
| A hook in the Claude Code settings | Prints the issues on stderr, and exits 2 when there is an issue |

In a project with `sgconfig.yml`, the hook also lints the files that the project's rules are for.
When a file holds a pipeline of steps, the hook also prints the errors and warnings of
[`xpd graph`](#step-code) for it. The hook exits 0 when there is no issue, and 1 when xpd cannot
run.

## lsp

`xpd lsp` starts the [language server](lsp.md) on stdin and stdout, so an editor can show the issues
of `check` as you type. It lints the text the editor holds, saved or not. It takes no flags, and it
reads one setting, `unsafeFixes`, from the editor.

| Exit code | Meaning |
| --- | --- |
| 0 | The editor sent `shutdown` and then `exit` |
| 1 | The editor went away without them, or never sent `initialize` |

## mcp

`xpd mcp` starts the [MCP server](mcp.md), and the arguments after `mcp` go to the server.

```console
$ xpd mcp -h
usage: xpd mcp [-h] [--root ROOT] [--code-mode | --no-code-mode]
               [--transport {stdio,http}] [--host HOST] [--port PORT]

The xpd MCP server. It serves stdio unless --transport says otherwise.

options:
  -h, --help            show this help message and exit
  --root ROOT           The project root. Every path a tool gets must be
                        inside it. Default: XPD_MCP_ROOT, else the working
                        directory. As an agent plugin, the default is the
                        first root the client names.
  --code-mode, --no-code-mode
                        Show the tools through FastMCP's code mode, as search,
                        get_schema and execute. Default: XPD_MCP_CODE_MODE,
                        else off.
  --transport {stdio,http}
                        Default: FASTMCP_TRANSPORT, else stdio.
  --host HOST           Address to bind for http. Default: FASTMCP_HOST, else
                        127.0.0.1.
  --port PORT           Port for http. Default: FASTMCP_PORT, else 8000.
```

The server is Python and installs with the xpd package. A binary built with cargo has no Python
beside it, so it prints `xpd: xpd mcp needs xpd installed as a Python package. Run it as: uvx xpd
mcp` and exits 2.

## skill

`xpd skill` prints the agent skill, `SKILL.md`, that ships with the installed xpd.

```sh
xpd skill [--json]
```

With `--json`, it prints the three skills that ship with xpd, `python-best-practices`,
`python-review` and `xpd`, as a list of objects with `name` and `files`. Each file has its `path`
in the skill folder and its `text`.

## completions

`xpd completions SHELL` prints the completion script for a shell, for every command and flag of
the installed xpd.

```sh
xpd completions bash
```

The shells are `bash`, `zsh`, `fish`, `powershell` and `elvish`.

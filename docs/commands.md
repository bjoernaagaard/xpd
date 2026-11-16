# Commands

`xpd -h` and `xpd <command> -h` always match the installed version.

## check

```sh
xpd check [PATHS]... [--format text|json|sarif|github] [--fix] [--unsafe-fixes]
```

Lints files and folders. Folders respect `.gitignore`. A library's rules run only on files
that import it, and the Python rules run on every file. A rule for a newer library version than the one installed in the project's venv
is skipped. `check` also warns once per run about a package the project's venv lacks, such as
pyarrow. See [Rules](rules.md).

Some rules have a fix, which is safe or unsafe. A safe fix keeps what the code does. An unsafe
fix may change it, for example because another name still points at the old object.

| Flags | Effect |
| --- | --- |
| none | Lint. `[*]` marks an issue with a safe fix |
| `--fix` | Write the safe fixes, then lint what is left |
| `--unsafe-fixes` | Lint. `[*]` also marks an issue with an unsafe fix |
| `--fix --unsafe-fixes` | Write every fix, then lint what is left |
| `--fix --review` | Review each safe fix before writing it |
| `--fix --review --unsafe-fixes` | Review safe and unsafe fixes before writing them |

Output is text on a terminal and JSON otherwise. The JSON has `fixed`, the number of fixes
written, `fixable`, the number of issues that `--fix` fixes with the same `--unsafe-fixes`
setting, and `issues`, each with `package`, `rule`, `file`, `line`, `column`, `severity`,
`message` and `fix`: `safe`, `unsafe` or `null`. A package warning is about the project and not
about a place in a file, so its `line` and `column` are `null`.

| Exit code | Meaning |
| --- | --- |
| 0 | No error-severity issue, after fixes |
| 1 | At least one error |
| 2 | xpd could not run, for example an unreadable path |

`scan` is an alias for `check`. SARIF output uses version 2.1.0 and includes rule descriptions,
source locations and proposed fixes. GitHub output uses workflow annotations. Neither format
changes the exit code or which rules run.

`--rule` selects a package, bare rule name or qualified name such as
`pandas/chained-assignment`. Repeat it to combine selections. `--severity warning` keeps
warnings and errors. `rules` accepts the same filters, and `rules --builtin` lists only the
bundled catalog.

A project adds rules through the nearest `sgconfig.yml` or `sgconfig.yaml`. `--config` selects
another configuration. `ruleDirs` and `utilDirs` contain plain ast-grep rules and shared
utilities. An optional `xpd.yml` beside the configuration assigns folders to library packs.
The files created by `xpd new project --pack team` show the format. Local fixes without a
safety declaration are unsafe. Library import gates, version bounds and value proofs apply
to local library packs just as they do to bundled packs. Plain structural rules run without
those Python checks.

A comment such as `# ast-grep-ignore: pandas/row-loop` suppresses the named rule on the next
line, or on the same line when trailing code. An unnamed suppression covers all rules.
The shared scanner reports comments that suppress no structural match as
`xpd/unused-suppression`. Filtered runs omit that check because excluded rules could use the
comment. Suppressions use public names, so they cover every variant of a rule.

The scanner currently infers languages from file extensions. It warns when a project sets
`customLanguages`, `languageGlobs` or `languageInjections`, which this scanner does not read.

## run

```sh
xpd run --lang python --pattern '$X.iterrows()' src --format json
xpd run --lang rust --kind function_item src
xpd run --pattern '$S.tz_convert("UTC").tz_localize(None)' --rewrite '$S.tz_convert(None)' analysis.py
```

Finds syntax patterns or node kinds without executing the source. The language comes from
`--lang` or each file's extension. Use `--selector` to select the matched part of a larger
pattern. `query` is an alias for `run`.

Without a write flag, a replacement is a preview. Use `--interactive` to accept individual
rewrites or `--update-all` to write every replacement. JSON output is a preview and cannot
be combined with either write flag. Structural rewrites have no xpd value
proof or safe-fix classification, so inspect the proposed changes before applying them.

JSON contains each match's file, text, byte range, line and column range, language and captured
variables. Positions in the structural JSON count from zero. Exit 0 means a match was found,
1 means no match and 2 means the command could not run.

## outline

```sh
xpd outline src --format json
xpd outline analysis.py --items all
xpd outline src --type function,class --match '^Data'
```

Reads symbols, imports, members, signatures and source ranges. Python outlines retain full
parameters, decorators and private names. The same extraction supplies xpd's Hamilton and step
analysis, while their dependency and column interpretation remains in xpd. Source indexing
also uses the extraction and keeps public alias and documentation resolution.

`--items structure`, `imports`, `exports` or `all` selects the entries. `--pub-members` keeps
public members. `--outline-rules` adds a YAML extraction file in ast-grep's outline schema.
`--no-default-outline-rules` uses only the supplied rules. No source code is run.

## new

```sh
xpd new
xpd new project --pack team --yes
xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes
xpd new test team/no-print --yes
xpd new util print-call --lang python --pattern 'print($X)' --yes
```

Without an item, `new` guides you through project, rule, case or utility creation. `--yes`
turns prompts off and requires the needed flags. `--directory` selects the project root.
`--format json` reports `createdFiles` and `modifiedFiles` as absolute paths. Existing rule
files are never overwritten.

A project has `sgconfig.yml`, `xpd.yml`, and rule, case and utility directories. Its configured
paths are relative. Rules belonging to a library pack use Python and run on files importing
that library, unless the pack has `--every-file`. Rules outside a pack can use any supported
language and do not have a library import gate.

A rule needs a valid `--pattern` and a `--message`. `--fix` also needs `--fix-safety safe` or
`unsafe`. `--since` and `--until` bound its library versions. `--holds VAR=RULE`,
`--unless-holds VAR=RULE` and `--unless-imported RULE` supply value proofs as YAML rules.
`--public-id` lets variants keep one reported name. The creator writes a documentation section
with the rule, and `--with-test` adds an empty case template. It does not invent expected cases.

Utilities are shared rules referenced by `matches: <name>`. `new help rule` and
`new rule --help` show the same command help.

## test

```sh
xpd test
xpd test --filter '^team/' --skip-snapshot-tests
xpd test --interactive
xpd test --engine structural --config sgconfig.yml
```

Reads the upstream suite shape with `id`, `valid` and `invalid` fields. The default xpd engine
runs examples through the same scanner as linting, including import gates, value proofs and
version selection. A case can supply `path` for path filters and source-language selection.
Valid examples must have no finding from their rule. Invalid examples must have a finding.

Snapshots record diagnostic ranges and proposed fixes. `--skip-snapshot-tests` checks only
validity. `--interactive` reviews changed snapshots, while `--update-all` accepts them.
Snapshots are stored in `__snapshots__` beside the suite, or the configured `snapshotDir`.
`--test-dir` overrides the directories in `sgconfig.yml`. `--format json` returns the outcomes
and failed count. Exit 0 means every example passed, 1 means an expectation failed and 2 means
the command could not run.

`--engine structural` uses ast-grep's upstream runner, suite format and snapshots. It checks
raw structural rules and does not apply xpd's library import gates or value proofs. Use the
default engine for rules whose expectations depend on xpd metadata.

## completions

```sh
xpd completions bash
```

Prints completion for the installed command schema, including `run`, `outline`, `new` and
`test`. Supported shells are `bash`, `zsh`, `fish`, `powershell` and `elvish`.

## profile

```sh
xpd profile data.csv
```

Prints JSON with each column's dtype, null count, unique count and memory use. Reads csv,
parquet, feather, json and xlsx. Runs with the project's Python and needs pandas there.

`findings` lists what the columns suggest about the file, each with the columns it is about and
a message that holds what was measured on the file:

| Finding | When | What the message names |
| --- | --- | --- |
| `values-as-column-names` | Three or more column names are years, are dates, or share a stem and differ in a number, as `sales_2022`, `sales_2023` and `sales_2024` do | The `melt` call that gives one row per value, with the other columns as `id_vars` |
| `category-candidate` | A text column has at most half as many distinct values as rows, and takes at most half the memory as a category | The bytes it takes as a category and as it is, and `dtype={"city": "category"}` |

```console
$ xpd profile sales.csv | jq -r '.findings[].message'
The column names 2019, 2020, 2021 and 3 more are values of one variable, a year, so the file is wide. df.melt(id_vars=["id", "city", "note"], var_name=..., value_name=...) gives one row per value, with the names in a column of their own.
city holds 3 distinct texts in 100,000 rows. As a category it took 100,044 bytes, where it takes 1,433,305 as str. Pass dtype={"city": "category"} to the reader, or call astype("category").
```

## refit

```sh
xpd refit model/contract.json
```

Refit loads the saved scikit-learn Pipeline with joblib, clones it and fits it on the raw
training rows. Run it in the project root with pandas and scikit-learn installed in the
project's Python. The contract must record `validation.splitter` with `class` naming a
splitter in `sklearn.model_selection` and `args` holding its literal keyword arguments.
It rebuilds the split, scores the held out rows with `get_scorer`, and computes fold scores
with `cross_val_score` on training rows only. It changes no saved file.

The JSON records actual row counts, `output`, `largest_gap`, both held out scores, both
lists of fold scores, and a match flag for each comparison. It compares `predict_proba`
when available, then `decision_function`, then `predict`. A nonnumeric prediction has a
null gap and is still compared. As in `equiv`, DataFrames and Series use pandas testing
defaults and other outputs use `numpy.testing.assert_equal`, which requires exact array
and score equality. Floating pandas values use the pandas default relative tolerance
of 0.00001 and absolute tolerance of 0.00000001. Save contract scores without rounding.
Exit 0 means all comparisons match, 1 means a comparison differs, and 2 means refit could
not run. Load only a model you trust because joblib loading runs code.

## equiv

```sh
xpd equiv old.py new.py
```

Runs both scripts and compares the variable `result` each one leaves. DataFrames and Series
are compared with `pandas.testing`, anything else with `numpy.testing`, so a list and an
array with the same values count as equal. Run it after you refactor data logic.

Each script runs in its own Python process, in the folder you run `xpd equiv` in. The second
script gets no module, no `sys.path` entry and no variable from the first. What a script
prints goes to stderr, so stdout holds only the JSON. A `result` travels back as a pickle, so
it cannot be an object of a class that the script itself defines.

| Exit code | Meaning |
| --- | --- |
| 0 | `{"equivalent": true}` |
| 1 | The results differ, with the difference in `detail` |
| 2 | A script failed, exited, has no `result` or one that pickle cannot save, or pandas is missing |

## graph

```sh
xpd graph [PATHS]... [--format text|json]
```

Prints the dataflow of Python files: which part of the code reads the result of which other
part, and what does not connect. It imports nothing and runs nothing, so it needs no Python.
It reads two ways of writing a dataflow, and picks one per file.

| A file with | Is read as |
| --- | --- |
| A function that passes a frame through other functions of the file | [Step code](#step-code) |
| Anything else | [Hamilton modules](#hamilton-modules) |

A file with a pipeline that also imports `hamilton` is read as both. So is a file whose only
pipelines are [method chains](#method-chains).

### Step code

A step is a function that takes a frame and returns it with columns added. A pipeline is a
function that runs steps in order, as `frame.pipe(a).pipe(b)` or as `frame = a(frame)` followed
by `frame = b(frame)`. `graph` reads from the source which columns each step reads and adds,
so it knows which step needs which.

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

The first line names the pipeline. Each line after it is `file:line: steps it reads from ->
step`, with the number of columns the step adds. `reads from its source` lists the columns the
steps read that no earlier step adds, so the frame the pipeline starts from must have them. The
list is written as the `columns=` argument of `pd.read_parquet`, which then reads no other
column. It ends in `and computed names` when a step also reads columns that the file does not
name. `order is free for` lists the steps that share no column with another step. Then come the
findings:

| Finding | Severity | What happens |
| --- | --- | --- |
| A step reads a column that a later step joins | error | The read raises `KeyError` |
| A step joins a column that an earlier step added | error | The join raises "columns overlap" |
| A step computes the formula of a column that another step adds | warning | The same work is done twice, and the two can drift apart |
| Two columns have the same formula | warning | The result holds one column twice |
| Two steps write out the same formula | warning | As above, with no column to read it from |
| A column is assigned twice in one step with the same formula | warning | The first assignment is wasted |
| A step writes to the frame it was given, as `frame["x"] = ...` | warning | Called as `step(frame)`, it changes the caller's frame |
| An `.assign(...)` of a chain reads a column from the name the chain starts at, as `orders["net"]`, and an earlier `.assign(...)` of the chain adds that column | warning | The name holds the frame from before the chain, so the read raises `KeyError`, or reads old values |
| The tree in the module docstring differs from the pipeline | warning | The description is out of date |
| A frame of new columns is not joined to the result | note | The result has no column of those names |

Two formulas are the same when they are equal after each local name is replaced by what it
stands for and the terms of a sum or a product are sorted. So `a + b` in one step equals `b + a`
in another.

What `graph` reads:

- A column read as `frame["name"]`, where the name is a literal, a constant of the module, or a
  list of those. `frame.name` counts when another step adds a column of that name.
- A column read as `pd.col("name")`, and as `d["name"]` in a lambda that `.assign(...)` calls
  with the frame, as in `frame.assign(total=lambda d: d["price"] * d["units"])`.
- A column added with `frame.assign(name=...)`, with `frame.join(new)` where `new` is
  `pd.DataFrame(index=frame.index).assign(name=...)`, or with `frame["name"] = ...`.
- A step at module level. A method of a class is not read.

A step whose column names the file computes, as in `frame[prefix + "_total"]` or a join of
`pd.get_dummies(...)`, is marked `and computed names`. Its named columns are still read, and it
is left out of `order is free for`, because not all of its columns are known.

The tree check applies to a file with one pipeline of functions whose module docstring draws
its steps as `├── step()` lines.

#### Method chains

A method chain is a pipeline too. Each `.assign(...)` of a chain with two or more steps is a
step, named by the columns it adds. The chain can stand in a function or in a statement of the
module, and it can mix `.assign(...)` with `.pipe(step)`. `two formulas are the same` holds
across the three ways to read a column, so `pd.col("a") * 2` in one step equals `d["a"] * 2` in
a lambda of another.

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
samples/graph/chain.py:17: warning: The column before_discount from .assign(net_amount, before_discount) has the same formula as gross_amount from .assign(gross_amount), so the result holds one column twice. Keep one of them.
samples/graph/chain.py:19: warning: .assign(customer_share) reads orders["net_amount"], and orders is the frame from before the chain, without the column net_amount that .assign(net_amount, before_discount) adds. The read raises KeyError, or reads old values when orders has such a column. Read it as pd.col("net_amount").
```

A pipeline in a statement of the module is named by the variable it is assigned to. A function
that another pipeline runs as a step is read as that one step, and its own chain is not listed
as a second pipeline. The functions of a file whose only pipelines are chains are also listed
as [Hamilton nodes](#hamilton-modules), as they are without the chains, so the lines
`orders -> features` and `inputs:` follow the pipelines in the full output of the file above.

### Hamilton modules

Prints the dataflow of [Apache Hamilton](https://hamilton.apache.org/) modules. Hamilton turns
every function in a module into a node: the function name is the output and the parameter names
are its inputs. `graph` reads the files and prints which node reads which. It imports nothing
and runs nothing, so it needs no Python and no Hamilton.

Pass the modules you give to one Driver. Together they are one dataflow, as with
`with_modules(loaders, features)`. A folder means every `.py` file in it. A script that builds
the Driver is not a node module, and its functions would show as nodes.

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

Each node line is `file:line: inputs -> node`. An input with `?` has a default. `inputs` lists
the names that no function defines, which the caller has to pass. `read by no node` lists the
ends of the dataflow. Then come the findings:

| Finding | Severity | What happens in Hamilton |
| --- | --- | --- |
| A function name defined twice in one file | error | Python keeps the last `def`, so the others never become nodes. Two `@config.when` versions need the names `rate__uk` and `rate__dk` |
| A node defined by two functions | error | The Driver raises "Cannot define function ... more than once" when it is built |
| A plain `def rate` beside `@config.when` on `rate__dk` | error | The same error, once the config selects the version |
| No type hint on the return value or on a parameter | error | The Driver raises "Missing type hint" when it is built |
| An input one edit from a node name | warning | If it is a typing error, execute raises "Required input ... not provided" |
| A decorator that `graph` cannot read | note | Nothing. The graph may lack nodes |

`graph` reads `@config.when` and its relatives, `@extract_columns`, `@extract_fields`,
`@unpack_fields`, `@parameterize`, `@parameterize_values`, `@parameterize_sources`, `@inject`,
`@load_from` and `@save_to`, when their arguments are literals. It prints a note for names that
the file computes, such as `@extract_columns(*COLUMNS)`, and for `@subdag`, `@pipe_input`,
`@pipe_output`, `@mutate`, `@with_columns`, `@does` and `@resolve`. It does not know the config
the Driver gets, so it lists every version of a node. It does not compare type hints between
nodes.

### Output and exit code

Output is text on a terminal and JSON otherwise. The JSON has `pipelines`, each with `name`,
`file`, `line`, `steps`, `links`, `source`, `source_complete` and `free`; `nodes`, each with `name`, `file`, `line`,
`function`, `inputs`, `kind` and `config`; `inputs`, each with `name`, `optional` and `readers`;
`unread`; and `problems`, each with `file`, `line`, `severity` and `message`. A step has `name`,
`kind`, `line`, `reads`, `columns` and `computed`, where `kind` is `function` or `link` for one
`.assign(...)` of a chain. A link has `from`, `to` and `columns`. `source` holds the columns the
pipeline reads from the frame it starts from, and `source_complete` is false when a step also
reads columns that the file does not name.

| Exit code | Meaning |
| --- | --- |
| 0 | No error |
| 1 | At least one error |
| 2 | xpd could not run, for example an unreadable path |

## hook

Reads a `PostToolUse` hook event on stdin, from Claude Code or from a client of the Agent Plugins
format, and lints the `.py` files the edit touched. When the file
holds a pipeline of steps, the hook also prints the errors and warnings of `xpd graph` for it. See
[Claude Code](claude-code.md).

## rules

```sh
xpd rules [--format text|json]
```

Lists every rule and package check of every pack, whatever the project's venv holds. Text has
one line per rule: its severity, `[package/rule]`, its fix and the first library version it
applies to when there is one, and its message.

The JSON is a list with one object per rule, each with `package`, `rule`, `severity`,
`message`, `fix` (`safe`, `unsafe` or `null`), `since` (a version or `null`), `url`, the
rule's link on [Rules](rules.md), and `doc`, that section as Markdown. A rule file may hold
several rule documents, one for each form the rule matches, and `rules` lists them as one rule
with the highest severity of its documents, the message of the first, the least safe fix, and
the lowest `since`, which is `null` when one document applies to every version.

## skill

```sh
xpd skill [--json]
```

Prints the agent skill that ships with this version of xpd. With `--json`, it prints all three
skills, `python-best-practices`, `python-review` and `xpd`, as a list of objects with `name`
and `files`, each file with its `path` in the skill folder and its `text`.

## mcp

```sh
xpd mcp [ARGS]...
```

Starts the [MCP server](mcp.md), which installs with xpd. The arguments go to the server, so
`xpd mcp -h` lists its flags.

## lsp

```sh
xpd lsp
```

Starts the [language server](lsp.md) on stdin and stdout, for an editor to run. It lints the
text the editor holds, saved or not, and reports what `check` reports for that text. It takes no
flags, and reads its one setting from the editor.

| Exit code | Meaning |
| --- | --- |
| 0 | The editor sent `shutdown` and then `exit` |
| 1 | The editor went away without them, or never sent `initialize` |

## find

```sh
xpd find "time series bootstrap for cross validation"
xpd find "grouped regression" --package sklego --kind symbol --format json
xpd find "small multiples" --kind rule
xpd find "rounded bars" --kind recipe
```

Returns separately ranked groups under the text headings `Symbols`, `Rules` and `Recipes`.
Defaults are up to five symbols, three rules and three recipes. `--limit N` overrides the
maximum for each kind. `--kind symbol`, `--kind rule` or `--kind recipe` limits the search
to that group. `--package` takes the Python import name. JSON always has the shape
`{"symbols": [...], "rules": [...], "recipes": [...]}`, with empty lists for excluded or
unmatched kinds. Empty matches exit 0. Each entry has a summary and rule links. Class
members are returned by `api`. The binary includes source indexes for all library packs,
worked recipes and rule documentation. No library installation is needed.

Discovery ranks the words in names, signatures, docstring summaries, rule texts and recipe titles. It also ranks by meaning, so "make the bars have soft corners" finds the recipe for rounded bars. It answers "what is the thing called that does X" but does not yet answer questions about a method's properties, such as which linear models need no scaling.

The ranking by meaning uses the EmbeddingGemma 300M model. The first `find` downloads it once,
333 MB, and prints a line on stderr before it starts. The download runs with the project's
Python, or else `python3` on `PATH`, and writes the file under a temporary name before it moves
it into place. The model is kept in `$XDG_CACHE_HOME/xpd/models` when `XDG_CACHE_HOME` is set.
Otherwise it is kept in `~/.cache/xpd/models` on Linux, `~/Library/Caches/xpd/models` on macOS
and `%LOCALAPPDATA%\xpd\models` on Windows. Later searches run offline, and loading the model and
embedding the query add about a quarter to half a second. When there is no Python, the download fails, or the model does not
load, `find` prints one line on stderr and ranks by keywords only. Search with `--index` ranks by
keywords only.

Names are split at underscores and camel case. Name tokens get weight three in BM25, with
uniform suffix stemming shared by queries and entries. Parameter names and
source documentation and class member summaries get lower weights. Exact acronym names and phrases get
bonuses, and shorter public paths rank higher. The build prepares the search table, and queries decode only the returned entries. Long signatures
are shortened in text search results; JSON and `api` retain every parameter.

With the model, the query is embedded and compared with a stored vector for each entry. The
vectors are the first 256 of the model's 768 values, which the model is trained to allow, and
`indexes/vectors.bin` holds them by entry name. Within each kind, the keyword ranking and the
ranking by meaning are combined by reciprocal rank fusion, and the top keyword matches get a bonus
so an exact match keeps its place. Results are matches, not a judgment that a library suits every
requirement. Read the chosen API and its rules before using it.

## api

```sh
xpd api sklego.meta.GroupedPredictor
xpd api plotly.express.bar --format json
```

Prints the entry's parameter names, first docstring paragraph, public methods and rule
links. JSON is a list containing one entry, with the same entry fields as `find`. Missing names exit
2. Parameter names come from source and stub files and do not include defaults or type
annotations. A class lists the parameters of its nearest `__init__`, or of its nearest `__new__`
when it has no `__init__`. A function that exists only as overloads in a stub file lists the
parameters of all its overloads. A class lists its public methods and properties, with the
inherited ones. A compiled function without a stub is absent. NumPy universal functions such as
`numpy.add` are absent, because their stubs declare them as attributes.

Rule links are inferred from symbol name tokens in the pack's rule patterns, including regex
patterns. They are possible guards, rather than proof that every use of the symbol matches.
The rule entries include the full documentation, so terms such as "small multiples" are searchable.
A rule prints as `plotly/bar-axis-without-zero [rule]`, without a call signature or raw matcher
metavariables. A recipe name combines the pack and the first eight words of its title. Recipes with the same name are deduplicated within a pack. The name stays the same when source paths or line numbers change. A recipe prints its title, used symbols, pinned source, license, attribution and
complete Python snippet. `recipe.modified` records whether xpd applied fixes; `recipe.code` is
null for a source whose license is not recognized as permitting redistribution.

## index

```sh
uv run --with griffe==2.4.1 xpd index /path/to/site-packages --package sklego
uv run --with griffe==2.4.1 xpd index /path/to/source/sklego --package sklego --version 0.9.10 --output local.json
xpd find "grouped regression" --index local.json
xpd api sklego.meta.GroupedPredictor --index local.json
```

Griffe 2.4.1 parses the library's Python source and stub files. Indexing never imports or
runs library code. Griffe is a build tool. Only `xpd index` needs it, and no other command
does. `xpd index` runs the Python of the active environment, so start it with
`uv run --with griffe==2.4.1`. Without that Griffe version, the command stops and prints the
command to run. The first command reads the version from wheel metadata and writes
`indexes/sklego.json`. For a source checkout, pass `--version`. `--output` selects another
file. The output directory must exist. Without `--examples`, the command keeps the recipes that
the output file already holds for the same package and version.
An explicit local index replaces the embedded index for its package and also supports libraries
without an xpd rule pack. Both read commands accept `--index`.

Each file has `schema_version`, `package`, `version` and `entries`. Every entry has a `kind`,
`package`, `name`, `callable`, `parameters`, `summary`, `members`, `rules` and optional
source-derived `keywords`. A recipe adds `recipe`, with `title`, `symbols`, `source`, `code`,
`license`, `attribution` and `modified`. An index with harvested examples also records candidate,
clean, fixed, dropped, capped and retained counts under `recipes`.

A name is public when Griffe says it is. A module is public when its name has no leading
underscore. Another name is public when its module lists it in `__all__`. In a module without
`__all__`, a name is public when it has no leading underscore and the module defines it.
The index adds four rules, and each one comes from a named symbol.

| Rule | Symbol that needs it |
| --- | --- |
| Names in a folder named `tests` are left out. | `pandas.tests.frame.test_api.TestDataFrameMisc` is in the wheel and is not library API. |
| A class or function that a module defines under a public name is public, even when `__all__` leaves it out. | `pandas.tseries.holiday.USFederalHolidayCalendar` |
| A name from `from module import *` is public, even when the importing module's `__all__` leaves it out. The module that the import reads must give the name. | `pandas.api.types.is_numeric_dtype` is public. `scipy.stats.interp1d` is not a name. |
| An import into a module without a readable `__all__` is public, when the name is defined in that module's package or below it. | `numpy.lib.stride_tricks.as_strided` is public. `pymoo.vendor.vendor_scipy.LocalSearch` is imported from another package and stays private. |

Each definition gets one entry for each public name it has. When several public paths reach
the same name, the entry uses the path with the fewest parts, then the shortest text, then
lexical order. `numpy.concat` and `numpy.concatenate` are two names for one function, and both
have an entry.

Griffe drops some callables that the library has. The index keeps them through a Griffe
extension and a Griffe loader subclass in `py/index.py`. Each case names its symbol.

| Case | Symbol |
| --- | --- |
| A function that exists only as overloads in a stub file | `numpy.zeros` |
| A plain assignment to another callable | `numpy.concat`, `pandas.io.pytables.Term`, `numpy.int64` |
| A function that its stub declares as an attribute | `numpy.polynomial.hermite.hermadd` |
| A class or function with the same name as the submodule that defines it | `numpy.memmap`, `scipy.optimize.least_squares` |
| An `__all__` that is computed and cannot be read | `scipy.signal.butter` |
| An import beside a stand-in definition that takes `*args, **kwargs` | `plotly.figure_factory.create_hexbin_map` |

To refresh recipes with symbols, use a pinned library checkout under a new empty `/var/tmp`
directory. For example, after downloading and unpacking the library source there, run one command.

```sh
uv run --with griffe==2.4.1 xpd index /var/tmp/xpd-source/scikit-lego-0.9.10 --package sklego --version 0.9.10 \
  --examples /var/tmp/xpd-source/scikit-lego-0.9.10 \
  --source-url https://github.com/koaning/scikit-lego/blob/v0.9.10 \
  --license /var/tmp/xpd-source/scikit-lego-0.9.10/LICENSE
```

`--examples` selects the documentation and gallery tree. The harvester reads Python docstring
Examples, Python files under examples or gallery directories, Markdown Python fences, RST
doctests and notebook code cells. For Plotly, select the repository's `doc/python` directory
and include `/doc/python` after the pinned revision in `--source-url`. For scikit-learn, select
the repository root to include both `examples/` and source docstrings. `--license` supplies the
source license. Root NOTICE, DISCLAIMER and third-party notices are retained automatically;
`--notice` adds a notice outside the selected root.

The harvester never imports or executes downloaded code. It rejects syntax errors, placeholders,
snippets without imports, missing names and snippets without calls into the requested library.
It runs the rule engine, applies safe and unsafe fixes, and rejects every remaining issue,
including informational issues. MIT, BSD, NCSA and Apache license text and copyright notices
are included with recipes. An unrecognized license yields a pointer with no copied code.
Parsing and linting do not prove that external datasets or optional dependencies are available.

Each pack retains at most 24 recipes, 96,000 code bytes and 8,000 bytes per snippet. Selection
prefers short examples of public APIs and different primary symbols. `capped` counts clean
examples omitted by those limits or because they repeat the same primary symbol. `dropped`
counts candidates rejected before selection. Refresh the source version and pinned URL together,
then rerun the command to regenerate both symbols and recipes.

`xpd find --check indexes/questions.json` runs the real discovery questions and exits 2 if an
expected entry is absent from the top five of its own kind, including five rules or recipes
regardless of the display defaults. It asks each question with keyword search, which is what
runs without the model, and again with the model when the model is there. A question marked
`"needs_model": true` is asked only with the model. An `expected` list names entries that each answer the question, and one of them in the top five passes. It prints every failure. A question marked `"open": true` records what a user asked and the answer they expect. It is kept as data for later work on discovery and is not checked. The check also exits 2 when an
index entry has no vector. After a change to `indexes/`, write the vectors again with
`xpd find --write-vectors indexes/vectors.bin`, which embeds every entry in about ten minutes. Lefthook runs this command
before commits with no tolerated misses. Questions that still fail go to a later ranking
investigation without changing their wording or adding question-specific ranking rules.

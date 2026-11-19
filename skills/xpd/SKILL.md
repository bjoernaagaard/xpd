---
name: xpd
description: "Python library habits: run xpd to lint code for slow or wrong use of libraries such as pandas and NumPy, profile data files, and prove refactors are equivalent."
---

# xpd

Use `xpd` whenever you write or change Python code.
`xpd <command> -h` lists the current flags.

- `xpd check <paths…> --format json`: lint. Fix every `error`. Fix `warning`s
  unless you can state why the pattern is right here. Don't silence rules.
  In a test file, `python/test-skip`, `python/test-rerun`, `python/test-sleep` and
  `python/test-suppressed-health-check` report a test that hides a failure. Find why the test fails, or delete it and write down why.
  A `pandas/missing-<package>` warning names a package the project lacks. Ask the user
  before you add a dependency.
- `xpd find "<task>"`: discover separately ranked symbols, rules and recipes before choosing a library.
  Defaults are five symbols, three rules and three recipes. `--limit` sets the maximum per
  kind and `--kind` selects one. JSON has `symbols`, `rules` and `recipes` lists.
  In code mode, read these lists directly. Record inspected candidates in the same grouped
  shape. A symbol read directly through `api` counts as a candidate when recorded there.
  `xpd api <dotted.name>` reads a result with its guarding rules and with each parameter as
  the source writes it, so it shows the type annotation and the default. A stub file writes
  `...` for a default, which says only that the parameter is optional. Read a method with
  `xpd api <Class>.<method>` before you call it, instead of importing the library to inspect it.
- `xpd rules --format json`: every rule with its documentation. Read a rule's
  `doc` before you change code it reports.
- `xpd doctor --format json`: the Ruff, ty and pytest settings worth changing in this
  project, each with its file, table, key, value and reason. A conditional one
  names the code that made it relevant and the condition the developer decides,
  so ask before you adopt it.
- `xpd check <paths…> --fix`: apply safe fixes. Add `--unsafe-fixes` for fixes
  that may change behavior, then read the diff.
- `xpd profile <data file>`: dtypes, nulls, cardinality, memory. Run it before
  choosing methods, so choices are based on the real data. Its `findings` name
  column names that hold values, which `melt` makes long, and text columns that
  suit `category`. Needs pandas.
- `xpd equiv old.py new.py`: both scripts must assign `result`. Run it after
  any refactor of data logic; a refactor isn't done until it prints
  `"equivalent": true`. Needs pandas. The old script is the judge, so don't edit it to
  make the two agree, and run both on real rows instead of a frame you typed in.
- `xpd graph <files…> --format json`: the dataflow of a file, read without
  running it. Run it after you add, move or rename a step or a node. Fix every
  `error`. Fix `warning`s unless you can state why the code is right.
  - Step code is a function that passes a frame through other functions, as
    `frame.pipe(a).pipe(b)`, or a method chain, where each `.assign(...)` is a
    step. Before you add a column, read `pipelines[].steps`
    and reuse a column that exists instead of computing its formula again.
    Pass `pipelines[].source` as `columns=` to `pd.read_parquet`, so the file
    is read without the columns that no step uses.
    Return `frame.assign(...)` or `frame.join(...)`, and don't write
    `frame["x"] = ...` on the frame a step was given.
  - Apache Hamilton modules: a `warning` names an input one edit from a node
    name, so check it for a typing error. `inputs` are what the caller passes.
- `xpd outline <paths…> --format json`: the imports, definitions and members of
  each file, with line numbers. Run it to map a project before you read whole
  files.
- `xpd run --pattern '<code>' <paths…> --json`: find code by its syntax, such as
  `'$X.iterrows()'`. Add `--rewrite '<code>'` to preview a replacement, and
  `--update-all` to write every match.
- `xpd refit <contract.json>`: refit a saved scikit-learn Pipeline on its raw
  training rows and compare outputs and scores with the contract. Needs pandas
  and scikit-learn.

When the xpd MCP server is connected, call its tools instead of the shell commands.
`check`, `fix`, `graph`, `profile`, `equiv`, `refit`, `find`, `api`, `outline`, `new` and
`test` match the commands of the same names, and `fix` is `check --fix`. `query` and
`rewrite` are `xpd run` without and with a replacement. They take the same paths and
return the same JSON. `rules` lists the rules without their documentation, and `rule`
returns one rule with it. `discovery_gate`, `contract` and `gates` judge a run from a
data file to a saved model.

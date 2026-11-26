---
name: xpd
description: "Python library habits: run xpd to lint code for slow or wrong use of libraries such as pandas and NumPy, profile data files, and prove refactors are equivalent."
---

# xpd

Use `xpd` whenever you write or change Python code.
`xpd <command> -h` lists the current flags.

- `xpd animate guide` or MCP `animate_guide`: start an editable Manim explanation from a
  brief or project concept. Check a low-cost preview before rendering the final video.
- `xpd check <paths…> --format json`: lint. Fix every `error`. Fix `warning`s
  unless you can state why the pattern is right here. Don't silence rules.
  In a test file, `python/test-skip`, `python/test-rerun`, `python/test-sleep` and
  `python/test-suppressed-health-check` report a test that hides a failure. Find why the test fails, or delete it and write down why.
  A `pandas/missing-<package>` warning names a package the project lacks. Ask the user
  before you add a dependency.
  With the xpd plugin installed, `xpd hook` lints each Python file you edit, and it holds the
  end of your turn while a file you changed has an `error` or a `warning`. When the code is right
  as it is, write `# ast-grep-ignore: <pack>/<rule>` on the line above, with the reason after it.
  A project or a user sets `style = "strict"` in `[tool.xpd]` of `pyproject.toml` or in
  `~/.config/xpd/config.toml`. The rules of the house style are then warnings: write a frame's
  steps as one method chain or as functions joined with `.pipe`, return `frame.assign(...)`
  instead of writing `frame["x"] = ...`, and follow the plotly pack in a chart.
  The hook also prints a brief of the project when a session starts, and the playbook that
  answers a prompt. Read that playbook with `xpd api` before you write the code.
- `xpd chart-review <file> --format json`: run it whenever you make,
  change or review a chart, in plotly, Dash, matplotlib, seaborn, pandas or any other library,
  also when the chart is one part of a larger task. It lists each chart with its kind and the
  plotly call that draws it, the plotly rules that fire, and the questions of Tufte's book that
  apply, each with its page, what to look at and the plotly code to write. Answer each question
  from the chart and its data. Redraw a chart of another library in plotly with its route, unless
  the user asks to keep the library. To measure the drawn figure, write an adapter whose
  `figures()` returns a dict of name to figure, such as a Dash callback called with the inputs
  of the state to review, and add `--run <adapter.py>`. It runs project code, so run it when the
  app can run. A failed run keeps the questions. For a chart drawn without Python, answer the
  questions of `xpd chart-review --questions` from the rendered chart.
- `xpd find "<task>"`: discover separately ranked symbols, rules and recipes before choosing a library.
  Defaults are five symbols, three rules and three recipes. `--limit` sets the maximum per
  kind and `--kind` selects one. JSON has `symbols`, `rules` and `recipes` lists.
  A recipe named `xpd/recipe/<name>` is a playbook, which is a complete module in the house
  style with the commands to run in order and the rules it satisfies. Start from `pandas-pipeline` for a
  frame pipeline, `feature-module` for model features, `linear-script` for a script that
  changes a frame line by line, and `plotly-chart` for a figure. `xpd api xpd/recipe/<name>`
  prints one.
  For a model run, record the inspected candidates in the same grouped shape. A symbol read
  directly through `xpd api` counts as a candidate when recorded there.
  `xpd api <dotted.name>` reads a result with its guarding rules and with each parameter as
  the source writes it, so it shows the type annotation and the default. A stub file writes
  `...` for a default, which says only that the parameter is optional. Read a method with
  `xpd api <Class>.<method>` before you call it, instead of importing the library to inspect it.
- `xpd rules --format json`: every rule with its documentation. Read a rule's
  `doc` before you change code it reports.
- `xpd doctor --format json`: how this project differs from xpd's profile of Ruff, ty and pytest
  settings and pytest plugins. `profile.changes` lists each change with its path, old and new
  value and reason, and `profile.diff` shows the edit to `pyproject.toml`. `suggestions` holds
  conditional advice, which names the code that made it relevant and the condition the developer
  decides, so ask before you adopt it. `xpd doctor --apply` writes the plan, so run it when the
  user asks for it.
- `xpd check <paths…> --fix`: apply safe fixes. Add `--unsafe-fixes` for fixes
  that may change behavior, then read the diff.
- `xpd profile <data file>`: dtypes, nulls, cardinality, memory. Run it before
  choosing methods, so choices are based on the real data. Its `findings` name
  column names that hold values, which `melt` makes long, and text columns that
  suit `category`. Needs pandas.
- `xpd equiv script.py --since HEAD`: compares a script you edited in place with
  its committed version. `xpd equiv old.py new.py` compares two files. Each script
  must assign `result`. Run it after any refactor of data logic; a refactor isn't
  done until it prints `"equivalent": true`. Needs pandas. The old script is the judge, so don't edit it to
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
- `xpd rules outline <paths…> --format json`: the imports, definitions and members of
  each file, with line numbers. Run it to map a project before you read whole
  files.
- `xpd rules run --pattern '<code>' <paths…> --json`: find code by its syntax, such as
  `'$X.iterrows()'`. Add `--rewrite '<code>'` to preview a replacement, and
  `--update-all` to write every match.
- `xpd untangle <paths…> --format json`: the ledger of a codebase refactor, read from the
  files and the git history without running anything. It counts copies, files that change
  together, import cycles, unused definitions and special cases, each finding with an `id`,
  its places and evidence. Run it before a refactor of a codebase and after each round, and
  add `--since <commit>` to see which findings are fixed and which are new since that
  commit. Copies and files that change together are found in every language, and the rest
  in Python. `xpd untangle start` begins a run that xpd records, with a plan, a gate for each fix
  and `xpd untangle status` for the stage that comes next. The `xpd-untangle` skill holds
  the steps.
- `xpd optimize <paths…> --format json`: the places where code does work that a shorter or
  faster program would not do, combining rulekit checks with whole-function and cross-place
  analysis without running code: rule issues, near copies, repeated calls and boolean masks,
  loop-invariant calls, default arguments, pairwise matrices built to be reduced or contracted,
  full materializations indexed for one element, and linear scans in loops, each with an `id`,
  its places and evidence. `xpd optimize start <paths> --check "<tests>" --bench "<workload>"`
  begins a recorded run that tracks code lines, benchmark timing and a gate for each fix, with
  `xpd optimize status` for the stage that comes next. Near copies are found in every language,
  and the rest in Python. The `xpd-optimize` skill holds the steps.
- `xpd refit <contract.json>`: refit a saved scikit-learn Pipeline on its raw
  training rows and compare outputs and scores with the contract. Needs pandas
  and scikit-learn.

The xpd MCP server holds only what a shell command cannot be, so run the commands above in
the shell also when the server is connected. Its `discovery_gate`, `contract` and `gates` tools
judge a run from a data file to a saved model, and `gates` runs the stages itself. Its three
`show_` tools draw what `xpd check`, `xpd graph` and `xpd profile` find, in a host that renders
apps.

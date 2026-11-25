Use `xpd find` before you choose a library or a function for a task. It searches the library
symbols, rules and recipes that ship with xpd for a task you describe in words, and it needs no
library installed. A recipe is a short worked example from a library's documentation, or one of
xpd's own [playbooks](#playbooks). Read an entry it returns with [`xpd api`](../api/) before you
write the call.

## Examples

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

Without `--kind`, the output has the headings `Symbols`, `Rules` and `Recipes`, each ranked on
its own. A symbol line shows the names of its parameters and its rule links.

## Playbooks

A playbook is a recipe of xpd's own, which holds the house style for a kind of task. It has the
asks it answers, a complete module that reports 0 issues under
[`style = "strict"`](../check/#house-style), the commands to run in order and the rules that the
code satisfies. `xpd api` prints all four parts.

| Playbook | What its code is | Asks it answers, for example |
| --- | --- | --- |
| `xpd/recipe/pandas-pipeline` | Step functions that each return `frame.assign(...)`, joined with `pipe` | "build a robust pandas pipeline" |
| `xpd/recipe/feature-module` | One typed function for each feature, named for the column it returns | "write a feature engineering module" |
| `xpd/recipe/linear-script` | A script that filtered and wrote a frame line by line, as one chain that `xpd equiv` proves equal | "turn this non-linear code into a linear pipeline" |
| `xpd/recipe/plotly-chart` | A line chart with a plain template, labels on the lines, and words and a unit on the axes | "improve my charts" |

A loose ask finds its playbook, because the asks of a playbook are part of the text that `find`
searches.

```console
$ xpd find "improve my charts" --kind recipe
Recipes
xpd/recipe/plotly-chart [recipe]
A plotly chart that passes the plotly pack and the chart review. A chart in the house style follows the plotly rules, which apply Edward Tufte's book "The Visual Display of Quantitative Information" to plotly figures, and answers the questions of the book that `xpd chart-review` asks for each chart. The template is plain and set once, each line carries its own label in place of a legend, the axes carry words and a unit, and the value axis starts at zero. A chart in matplotlib, seaborn or another library gets the same review, with the plotly call that draws it.
Rules: plotly/default-template, plotly/legend-instead-of-labels, plotly/axis-title-column-name, plotly/bar-axis-without-zero, plotly/area-axis-without-zero, plotly/explicit-grid, plotly/boxed-axis-frame, plotly/redundant-color, plotly/rainbow-colorscale, plotly/pie-chart, plotly/rotated-tick-labels, plotly/all-caps-title
plotly/recipe/scatter-line-area-and-bar-charts [recipe]
Scatter, Line, Area and Bar Charts
plotly/recipe/polar-chart-with-plotly-express [recipe]
Polar chart with Plotly Express
```

Between two recipes with the same score, a playbook comes first. With the xpd plugin installed,
[`xpd hook`](../hook/#on-each-prompt) runs this search on each prompt and prints the playbook for
the agent, so nobody has to ask for it. The playbooks are the Markdown files in
`rules/xpd/playbooks` of the repository.

## Ranking

`find` ranks each group by keywords and by meaning, so "make the bars have soft corners" finds
the recipe for rounded bars. The meaning comes from the EmbeddingGemma 300M model. The first
`find` downloads the 333 MB model once with the project's Python, and later searches run
offline. Without Python, without the download or with `--index`, `find` ranks by keywords only
and prints one line on stderr that says why.

| System | Where the model is kept |
| --- | --- |
| `XDG_CACHE_HOME` is set | `$XDG_CACHE_HOME/xpd/models` |
| Linux | `~/.cache/xpd/models` |
| macOS | `~/Library/Caches/xpd/models` |
| Windows | `%LOCALAPPDATA%\xpd\models` |

A match means the entry fits the words of the query. Class members are not results, so use
`api` to read them.

## Discovery questions

`--check FILE` runs the search questions in a JSON file, each with the entries it should find,
and fails when an expected entry is not in the top five of its own kind. It also fails when the
code of a playbook has an issue under the strict style. The repository checks
`indexes/questions.json` before each commit that changes an index.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The search ran, even when nothing matched, or every question of `--check` passed |
| 2 | xpd could not run, a question of `--check` failed, a playbook has an issue, or an index could not be read |

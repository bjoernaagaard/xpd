Use `xpd find` before you choose a library or a function for a task. It searches the library
symbols, rules and recipes that ship with xpd for a task you describe in words, and it needs no
library installed. A recipe is a short worked example from a library's documentation. Read an
entry it returns with [`xpd api`](../api/) before you write the call.

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
and fails when an expected entry is not in the top five of its own kind. The repository checks
`indexes/questions.json` before each commit that changes an index.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The search ran, even when nothing matched, or every question of `--check` passed |
| 2 | xpd could not run, a question of `--check` failed, or an index could not be read |

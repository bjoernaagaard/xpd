Run `xpd find "<task>"` before choosing a library function to search xpd's bundled library symbols, project-local libraries configured in `[tool.xpd.libraries]` via [`xpd library`](../library/), rules, worked recipes, and playbooks without installing those libraries. Read any chosen entry in full with [`xpd api`](../api/).

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

```console
$ xpd find "improve my charts" --kind recipe
Recipes
xpd/recipe/plotly-chart [recipe]
...
```

## Playbooks

A playbook is a complete house style module that passes [`style = "strict"`](../check/#house-style) with ordered commands and rules. Playbooks rank first on ties and are surfaced automatically by [`xpd hook`](../hook/#on-each-prompt).

| Playbook | Pattern | Sample query |
| --- | --- | --- |
| `xpd/recipe/pandas-pipeline` | Step functions returning `frame.assign(...)` joined with `pipe` | `"build a pandas pipeline"` |
| `xpd/recipe/feature-module` | One typed function per feature column | `"write a feature engineering module"` |
| `xpd/recipe/linear-script` | Step-by-step script rewritten as one chain verified by `xpd equiv` | `"turn this script into a linear pipeline"` |
| `xpd/recipe/plotly-chart` | Tufte-compliant plotly figure with direct labels and plain template | `"improve my charts"` |

## Hybrid ranking and offline cache

`xpd find` ranks `Symbols`, `Rules`, and `Recipes` by combining keyword scores with semantic embeddings from EmbeddingGemma 300M (downloaded once, 333 MB). Without Python or the cached model, `xpd find` falls back to keyword ranking and notes the fallback on stderr.

| Platform | Embedding model cache |
| --- | --- |
| `XDG_CACHE_HOME` set | `$XDG_CACHE_HOME/xpd/models` |
| Linux | `~/.cache/xpd/models` |
| macOS | `~/Library/Caches/xpd/models` |
| Windows | `%LOCALAPPDATA%\xpd\models` |

Pass `--check <file>` (such as `indexes/questions.json`) to assert that every benchmark query returns its expected entry in the top five and every playbook passes strict linting.

## Exit status

| Exit code | Meaning |
| --- | --- |
| `0` | Search ran (even with zero matches) or all `--check` questions passed |
| `2` | Invalid arguments, unreadable index, failing `--check` question, or playbook lint error |

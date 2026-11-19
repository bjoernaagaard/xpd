Use `xpd index` when [`xpd find`](../find/) and [`xpd api`](../api/) should search a library or a
version that xpd does not ship. It builds a search index from the Python source and stub files
of a library, and never imports or runs library code. It parses the source with Griffe 2.4.1 in
the project's Python, so start it with `uv run --with griffe==2.4.1`. No other command needs
Griffe.

## Examples

```sh
uv run --with griffe==2.4.1 xpd index /path/to/site-packages --package sklego
uv run --with griffe==2.4.1 xpd index /path/to/source/sklego --package sklego --version 0.9.10 --output local.json
xpd find "grouped regression" --index local.json
xpd api sklego.meta.GroupedPredictor --index local.json
```

Without Griffe, `xpd index` stops and prints the command to run.

```console
$ xpd index samples/sklego --package sklego --version 0.9.10
xpd: Index building needs griffe==2.4.1. Run uv run --with griffe==2.4.1 xpd index samples/sklego --package sklego --version 0.9.10.
```

A local index replaces the shipped index of its package, and it also works for a library without
an xpd rule pack. Without `--examples`, the command keeps the recipes that the output file
already holds for the same package and version.

## Index file

Each index file has `schema_version`, `package`, `version` and `entries`. `schema_version` is 2,
and `find` and `api` refuse a local index of schema 1.

| Key | What it holds |
| --- | --- |
| Every entry | `kind`, `package`, `name`, `callable`, `parameters`, `summary`, `members` and `rules`, and `keywords` taken from the source when there are any |
| An entry or a member with parameters | `parameters_written`, the parameter list as the source writes it |
| A member | `name`, `parameters` and `summary`, and `property`, which is `true` for a property |
| A recipe entry | `recipe`, with `title`, `symbols`, `source`, `code`, `license` and `modified`, and `attribution` when the source file has copyright lines |
| `recipes` | The counts from collecting recipes, with `candidates`, `clean`, `fixed`, `dropped`, `capped` and `retained` |

## Recipes

`--examples` collects recipes from the documentation of a library checkout pinned to a version.
It reads the Examples sections of docstrings, Python files under examples or gallery folders,
Python code blocks in Markdown, doctests in reStructuredText and notebook code cells, and it
never runs them. It keeps a snippet only when the rule engine finds no issue in it after fixes,
and it keeps up to 24 recipes and 96,000 code bytes per pack. `--license` names the license of
each recipe, and a license that xpd does not recognize gives a pointer to the source without
the code.

```sh
uv run --with griffe==2.4.1 xpd index /var/tmp/xpd-source/scikit-lego-0.9.10 --package sklego --version 0.9.10 \
  --examples /var/tmp/xpd-source/scikit-lego-0.9.10 \
  --source-url https://github.com/koaning/scikit-lego/blob/v0.9.10 \
  --license /var/tmp/xpd-source/scikit-lego-0.9.10/LICENSE
```

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The index was written |
| 2 | Griffe is missing, the source cannot be read, or the output folder does not exist |

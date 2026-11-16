# xpd

xpd checks Python code for slow operations and wrong results. It checks code that uses pandas,
NumPy, SciPy, scikit-learn, scikit-lego, MAPIE, Category Encoders, pymoo or Apache Hamilton. xpd
can rewrite many of the lines it reports.

xpd reads files in parallel and parses each file once with
[ast-grep](https://ast-grep.github.io/). Linting starts no Python, so a coding agent can run xpd
after every edit.

## Install

```sh
uv tool install xpd
```

Or run it once without installing:

```sh
uvx xpd check .
```

## First run

```sh
xpd check .
```

xpd prints one line per issue and a summary. Fix every `error`. Fix a `warning` unless you can
say why the pattern is right in that spot.

`xpd check . --fix` applies safe fixes. `xpd check . --fix --unsafe-fixes` also applies fixes
that may change behavior, so read the diff afterwards.

## Where to go next

| Page | What it covers |
| --- | --- |
| [Commands](commands.md) | Every command, its output and its exit codes |
| [Claude Code](claude-code.md) | The hook that lints each edit an agent makes, and xpd as an agent plugin |
| [MCP server](mcp.md) | The tools, resources and prompts an agent gets from `xpd mcp` |
| [Code mode](code-mode.md) | A first draft of a run from a data file to a saved scikit-learn model, in one call |
| [Language server](lsp.md) | The issues, fixes and rule documentation an editor gets from `xpd lsp` |
| [Rules](rules.md) | Every rule, with code before and after |

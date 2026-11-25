Each page below lists the arguments and options of one command, with their defaults, as the
installed xpd reads them. The site builds these lists from xpd's own command definitions, so
they change when xpd changes. `xpd -h` and `xpd <command> -h` print the same flags in a terminal,
and every command takes `-h` and `--help`. `xpd -V` prints the version.

Start with [`xpd check`](./check/), which lints Python files and writes fixes.

## The project's Python

`profile`, `equiv`, `refit`, `chart-review --run` and `index` run Python code, and `find` uses Python once to download
its search model. xpd uses the first of these Pythons that it finds.

1. The one in `$VIRTUAL_ENV`
2. The one in the nearest `.venv` at or above the current folder
3. `python3` on `PATH`

## Exit status

Most commands exit 0 when they succeed and 2 when xpd cannot run, for example because a file
cannot be read. A command that checks something, such as `check`, `graph`, `rules run`, `rules test`,
`equiv` and `refit`, exits 1 when the check finds a problem. Each command page has the details.

Use `xpd skill` to give an agent the instructions that go with the installed xpd. It prints the
agent skill, `SKILL.md`. With `--json`, it prints the bundled skills as a list of objects with `name` and
`files`, where each file has its `path` in the skill folder and its `text`.

## Examples

List the bundled skill names.

```console
$ xpd skill --json | jq -r '.[].name'
python-best-practices
python-review
xpd
xpd-animate
xpd-chart-review
xpd-data-prep
xpd-harden
xpd-optimize
xpd-untangle
```

## Exit status

It exits 0.

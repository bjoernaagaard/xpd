An agent can read every rule and every skill of xpd as a resource. A client can ask the server to
complete the `package` and `rule` parts of a template, so the user picks a value from a list
while typing. The server offers only the rules of the package that is already chosen, so `rule`
with the text `chained` in the pandas pack completes to `chained-assignment`.

## xpd://rules

Every rule as a JSON list, with the same fields as `xpd rules --format json` and without `doc`.
It lists the project's own rules as well, and reads them again after the project's
`sgconfig.yml`, `xpd.yml` or a rule file has changed.

## xpd://version

The resource holds the version and nothing else, such as `0.38.0`.

## skill://python-best-practices/

The Python style skill, which holds the style that a linter cannot check.

## skill://python-review/

The Python review skill, with the template of the review report.

## skill://xpd/

The skill that `xpd skill` prints. Each skill's `_manifest` lists its files with their size and
SHA-256 hash, and clients that support the Skills extension list the skills.

## ui://

The page that draws the views of the `show_issues`, `show_dataflow` and `show_columns` tools in
hosts that render MCP Apps. Its address holds a hash of the renderer, so it changes when the
renderer changes. It loads its scripts from `cdn.jsdelivr.net`.

## xpd://rules/{package}

The rules of one pack, such as `xpd://rules/pandas`, as a JSON list.

## xpd://rules/{package}/{rule}

One rule's documentation as Markdown, the same text as its [rule page](../../rules/) and the
`doc` that `xpd rules` prints. `xpd://rules/pandas/no-inplace` starts like this.

````markdown
`inplace=True` gives no speedup and keeps the call out of a method chain. Assign the result instead.

```diff lang="python"
-df.dropna(inplace=True)
+df = df.dropna()
```
````

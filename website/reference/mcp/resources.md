An agent can read every rule and every skill of xpd as a resource. A client can ask the server to
complete the `package` and `rule` parts of a template, so the user picks a value from a list
while typing. The server offers only the rules of the package that is already chosen, so `rule`
with the text `chained` in the pandas pack completes to `chained-assignment`.

## xpd://rules

Every rule as a JSON list, with the same fields as `xpd rules --format json` and without `doc`.
It lists the project's own rules as well, and reads them again after the project's
`sgconfig.yml`, `xpd.yml` or a rule file has changed.

## xpd://version

The resource holds the installed xpd version and nothing else.

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

## skill://xpd-animate/

The offline animation workflow and its editable Manim recipes.

## skill://xpd-optimize/

The steps from a finding of [`xpd optimize`](../../commands/optimize/) to a change with proof that
the result is the same.

## skill://xpd-untangle/

The steps of a refactor of a whole codebase, which [`xpd untangle`](../../commands/untangle/) measures.

## xpd://animation/{job_id}/video.mp4

The video of a local preview or final render.

## xpd://animation/{job_id}/still.png

The last frame from a local still render.

## xpd://animation/{job_id}/contact-sheet.png

A set of sampled frames from a local video render.

## xpd://animation/{job_id}/source.zip

The editable scene and its saved inputs.

## xpd://animation/{job_id}/manifest.json

The settings and metadata of a local render job.

## xpd://animation/{job_id}/render.log

The saved log of a local render job.

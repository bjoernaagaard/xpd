Use `xpd model-review gate` in a [recorded model review](../../model-review/#recorded-stages).

Commit the fix and pass its ID. It must preserve passing checks and avoid increasing the finding count. A cited finding must disappear. A design fix uses `"goal": "design"`, cites `questions` and edits a file owning a cited question.

## Example

```console
$ xpd model-review gate --help
Gate a fix that is committed: run the checks, time `--bench` when set, count lines and compare the findings with the commit before it. Exit 1 when the fix is not kept
...
```

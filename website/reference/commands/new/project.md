Use `xpd new project` to start a folder of your own rules. It writes `sgconfig.yml`, `xpd.yml`,
and the `rules`, `rule-tests` and `utils` folders, with relative paths. Each `--pack` declares a
pack whose rules run on the files that import the module, or on every Python file with
`--every-file`. It stops when the folder already has `sgconfig.yml`.

With `--format json` it prints the files it wrote. In a folder named `/tmp/project`, that looks
like this.

```console
$ xpd new project --pack team --yes --format json
{"item":"project","createdFiles":["/tmp/project/sgconfig.yml","/tmp/project/xpd.yml","/tmp/project/rules/.gitkeep","/tmp/project/rule-tests/.gitkeep","/tmp/project/utils/.gitkeep","/tmp/project/rules/team/.gitkeep"],"modifiedFiles":[]}
```

Use `xpd new project` to start a folder of your own rules. It writes `sgconfig.yml`, `xpd.yml`,
and the `rules`, `rule-tests` and `utils` folders, with relative paths. Each `--pack` declares a
pack whose rules run on the files that import the module, or on every Python file with
`--every-file`. It stops when the folder already has `sgconfig.yml`.

```sh
xpd new project --pack team --yes
```

With `--format json` it prints the files it wrote. In a folder named `/work/rules-demo`, that
looks like this.

```json
{"item":"project","createdFiles":["/work/rules-demo/sgconfig.yml","/work/rules-demo/xpd.yml","/work/rules-demo/rules/.gitkeep","/work/rules-demo/rule-tests/.gitkeep","/work/rules-demo/utils/.gitkeep"],"modifiedFiles":[]}
```

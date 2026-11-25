Use `xpd rules new util` to add a utility rule, which any rule of the project can use with
`matches: <name>`. It goes to the first folder that `utilDirs` in `sgconfig.yml` names.

```console
$ xpd rules new project --yes --format json | jq -c .item
"project"
$ xpd rules new util print-call --lang python --pattern 'print($X)' --yes
Created util print-call
  created  /tmp/project/utils/print-call.yml
```

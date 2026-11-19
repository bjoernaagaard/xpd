Use `xpd new util` to add a utility rule, which any rule of the project can use with
`matches: <name>`. It goes to the first folder that `utilDirs` in `sgconfig.yml` names.

```sh
xpd new util print-call --lang python --pattern 'print($X)' --yes
```

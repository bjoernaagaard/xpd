Use `xpd new test` to add an empty test case for a rule, by its id or its public id. It writes no
valid or invalid code, so add examples and run [`xpd test`](../../test/).

```console
$ xpd new project --pack team --yes --format json | jq -c .item
"project"
$ xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --yes --format json | jq -c .id
"team/no-print"
$ xpd new test team/no-print --yes
Created test team/no-print
  created  /tmp/project/rule-tests/team/no-print-test.yml
```

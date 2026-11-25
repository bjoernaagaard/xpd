Use `xpd rules new test` to add an empty test case for a rule, by its id or its public id. It writes no
valid or invalid code, so add examples and run [`xpd rules test`](../../test/).

```console
$ xpd rules new project --pack team --yes --format json | jq -c .item
"project"
$ xpd rules new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --yes --format json | jq -c .id
"team/no-print"
$ xpd rules new test team/no-print --yes
Created test team/no-print
  created  /tmp/project/rule-tests/team/no-print-test.yml
```

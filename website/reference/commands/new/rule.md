Use `xpd new rule` to add a rule, with the Markdown that explains it beside the rule file. A rule
in a pack is a Python rule named `<pack>/<name>`, and runs only on the files that import the
pack's module. A rule outside a pack is a plain rule for the language that `--language` names.

A rule needs a `--pattern` and a `--message`, and `--fix` needs `--fix-safety`. Before it writes
the rule file, `new` checks that both `xpd check` and `ast-grep scan` can read the rule. It never
overwrites a rule file.

```sh
xpd new rule team/no-print --pattern 'print($X)' --message 'Use the project logger.' --with-test --yes
```

The rule's Markdown goes to `rules/team/no-print.md`, the same place where xpd's own packs keep
the text of their [rule pages](../../../rules/).

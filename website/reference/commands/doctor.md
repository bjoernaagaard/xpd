Use `xpd doctor` when you set up Ruff and ty for a project. It suggests the settings worth
changing in the project in the working folder. Each suggestion names the file and table it goes
in, the key and value, the reason, and whether an installed checker confirmed it. Nothing is
installed or edited.

The suggestions come from a review of every rule and setting of Ruff 0.17.4 and ty 0.0.106,
which ships inside xpd. A recommended entry is suggested wherever it is not already in effect. A
conditional one is suggested only when the project's code or configuration shows that it
applies, with the condition that the developer decides and the lines that made it relevant. A
rule that the project turns off on purpose is listed under "Project choices doctor keeps".

Ruff and ty are optional. Doctor runs one that is in the project's venv or on `PATH` to read the
settings it resolves. Without one, its suggestions rest on the reviewed release and say
"unverified locally".

## Examples

```sh
xpd doctor --format text
xpd doctor --format json | jq '.suggestions[] | {tool, key, value}'
```

The JSON has `tools`, `suggestions`, `policies` and `notes`. Each suggestion has `record`,
`tool`, `id`, `decision` (`recommend` or `conditional`), `file`, `table`, `key`, `value`,
`current`, `edit`, `scope`, `reason`, `source`, `condition`, `evidence` and `status`.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The suggestions were printed |
| 2 | xpd could not run |

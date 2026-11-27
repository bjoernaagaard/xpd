Run `xpd discover` to find the names, strings and modules that one text search does not find, and gate each rename in a recorded run.

| Group | Finding kinds |
| --- | --- |
| Names that one search does not find | `name-collision`, `generic-verb-name`, `vague-module-name`, `module-private-name` |
| Strings and dead ends that a search misses | `interpolated-key`, `error-message-prefix`, `deprecated-without-decorator` |
| Types that let a caller swap arguments | `id-without-newtype` |

A `name-collision` finding is a public function or class name that two or more modules define. Test files, `__init__.py`, Hamilton modules, `main` and names that start with an underscore are left out. The other kinds come from the Python rules of the same name. The header counts how many public names one module alone defines, and a rename that makes a name collide adds a finding that fails the gate.

Each module with a finding gets six checklist items. Run `xpd discover --questions` to read the questions and the facts that inform them.

## Recorded run

1. **Start**: record the current commit, paths, and project checks from a clean tracked tree with `xpd discover start titles.py notes.py --check "your-project-check"`.
2. **List**: read stable finding IDs and checklist item IDs with `xpd discover . --format json`.
3. **Plan**: record ordered fixes, accepted findings, and checklist verdicts (`pass`, `na`, or `accepted`) with reasons using `xpd discover plan plan.json`. Set `changes_interface` on a rename that other modules import.
4. **Approve**: record approval for an interface change with `xpd discover approve fix-id`.
5. **Gate**: commit one fix and run `xpd discover gate fix-id`.
6. **Drop**: return to the run's current commit and record a reason with `xpd discover drop fix-id --reason "reason"`.
7. **Status**: show the current run, its recorded answers, and the next stage with `xpd discover status`.
8. **Finish**: compare against the starting commit with `xpd discover finish`. An open fix, a new finding without an accepted reason, or an unanswered checklist item causes exit 1.

## Example

Two modules define `clean`, so a search for it finds two definitions, and neither name says what it cleans.

```python title="titles.py"
def clean(text):
    return text.strip()
```

```python title="notes.py"
def clean(text):
    return " ".join(text.split())
```

```console
$ git init -q && git add titles.py notes.py && git -c user.name=xpd -c user.email=xpd@example.com -c commit.gpgsign=false commit -qm start
$ xpd discover start titles.py notes.py --check "python3 -m py_compile titles.py notes.py" --format json | jq -c '{findings: .run.was, next: .next.stage}'
{"findings":3,"next":"plan"}
$ xpd discover titles.py notes.py --format json | jq -c '{names: .read, kinds: [.findings[].kind]}'
{"names":{"files":2,"public_names":1,"defined_once":0},"kinds":["name-collision","generic-verb-name","generic-verb-name"]}
$ xpd discover titles.py notes.py --format json | jq -c '{fixes: [{id: "rename-clean", title: "Name each clean after what it cleans", findings: [.findings[].id], changes_interface: true}]}' | xpd discover plan - --format json | jq -c .next.stage
"ask"
$ xpd discover approve rename-clean --format json | jq -c .next.stage
"fix"
$ sed -i 's/^def clean(/def clean_title(/' titles.py && sed -i 's/^def clean(/def clean_note(/' notes.py && git -c user.name=xpd -c user.email=xpd@example.com -c commit.gpgsign=false commit -qam "Name each clean after what it cleans"
$ xpd discover gate rename-clean --format json | jq -c '.run.rounds[0][0] | {outcome, fixed: (.fixed | length)}'
{"outcome":"kept","fixed":3}
$ xpd discover titles.py notes.py --format json | jq -c .read
{"files":2,"public_names":2,"defined_once":2}
$ xpd discover finish --format json | jq -c .run.finished
xpd: 3 findings at the start, 0 now: 3 fixed, 0 added, 0 left without a reason; lines 4 -> 4
true
```

## Exit status

A successful list or recorded stage exits 0. A rejected gate or a finish blocked by findings or checklist items exits 1. Invalid input exits 2.

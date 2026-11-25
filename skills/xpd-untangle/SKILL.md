---
name: xpd-untangle
description: Refactor a whole codebase with xpd untangle, which measures where the code gets wrong which pieces change together. Use when the user asks to untangle, restructure, decouple or clean up a codebase, to remove dead code, duplication or special cases across a project, or to find what to refactor first.
---

# Untangle a codebase with xpd

Use this guide for a refactor of a whole codebase or of one of its packages. For a change to
one function or one script, the `xpd` skill is enough.

A codebase is well structured when it answers one question right: **should these pieces change
together?** There are three ways to get it wrong, and `xpd untangle` measures each of them. It
reads the files and the git history, and runs no code.

| Group | What is wrong | Kinds that `xpd untangle` counts |
| --- | --- | --- |
| Hidden link | Pieces must change together, and the code does not show it | `copy`, `changes-together`, `shared-state` |
| Needless link | Pieces are tied together and change for different reasons | `volatile-import`, `import-cycle`, `private-reach`, `vague-module` |
| Surplus | A piece has no reason of its own to change | `pass-through`, `unreferenced`, `constant-parameter` |
| Special case | Code exists for one situation, which the design never accounted for | `one-value-branch`, `one-caller-parameter`, `swallowed-exception`, `sleep-or-retry`, `disabled-test` |

`copy` and `changes-together` are found in every programming language. The other kinds are
read from Python. In another language, find them by reading, with the table as the list.

The refactor is a run with stages, and no stage grades its own work. You judge, design and
edit. xpd records what was decided, says which stage comes next, and gates each fix itself
with the project's checks and the ids of the findings.

```text
xpd untangle start <paths> --check "<command>"     record the commit, the paths and the checks
repeat
  xpd untangle --format json                       read the ledger
  xpd untangle plan <file>                         the fixes in order, and the accepted findings
  for each fix
    edit, commit
    xpd untangle gate <fix>                        exit 0 kept, exit 1 not kept
until no finding is left that is worth a fix
xpd untangle finish                                compare with the commit of the start
```

The run is a journal in the git folder of the repository, `.git/xpd-untangle.json`, so it
survives the end of a session and needs no agent tool. **After a break, run
`xpd untangle status`.** It prints the run and the stage that comes next, with what to do.
Where your tool has subagents, give the design and each fix to separate agents. The gate is
xpd either way.

## Start

`xpd untangle start <paths> --check "<command>"` records the commit that is checked out, the
paths and each check, and runs each check once. Give a `--check` for each command the project
checks itself with, such as its tests, its type checker and its linter. A check must exit 0
only when the project is sound, need no input and change no tracked file. A check that fails
at the start gates nothing. The tracked files must match the commit.

## Read the ledger

`xpd untangle --format json` gives `summary`, with the count of each kind, and `findings`, the
strongest of each kind. Each finding has an `id`, its `kind`, its `places` and the `evidence`
that was measured. `--kind <kind> --limit 0` lists every finding of one kind.
`xpd rules outline <paths…>` gives the skeleton of the files a finding names, so read that
before whole files.

## Judge

A finding is evidence, and you decide what it means. Read the code at its places.

- Two files that change together may be a feature split across folders, a copy of a value, or
  a test beside its subject.
- A branch for one value may be a rule of the domain, or one of twelve places that each know
  about one provider.
- A definition that nothing mentions may be called from another language or from outside.

Accept a finding that is right as it is, with its `id` and the reason in one sentence. The
journal keeps it, so a later round does not judge it again. Every other finding goes under a
fix, or waits for the next round.

## Redesign

Work the fixes out as if the system had known today's requirements from the start. Group the
findings that have one cause, and write the fix above them. One fix that removes several
findings often looks like none of them: a registry in place of five lists, an interface in
place of a branch for each provider, one module in place of two that always change together.
Do not fix special cases one by one.

Ask the same of every fix, also one that looks better in the code.

- Pieces that change together end up together.
- Pieces that do not change together are not tied.
- Every import points at code that changes less.

Design no fix that removes what a check runs, such as the last test a test command collects,
because the gate then fails. Accept that finding instead, and say that the check stands in
the way.

Write the plan as JSON and run `xpd untangle plan <file>`, or `plan -` to read it from stdin.

```json
{
  "fixes": [
    {
      "id": "pricing-owns-prices",
      "title": "Move every price rule into shop/pricing.py",
      "findings": ["d81b64d1", "8de3d24b"],
      "changes_interface": false
    }
  ],
  "accepted": [
    {"finding": "cc6bc6e7", "reason": "The shared commits only append comment lines."}
  ]
}
```

The fixes are made in the order of the plan, each on the result of the one before. Set
`changes_interface` when a fix changes what code outside the paths can call, or what a user of
the program sees. xpd refuses a plan that names a finding the ledger does not have.

## Fix and prove

Make the fix that `xpd untangle status` names, and only that one. The behavior stays the
same, apart from a bug the fix removes, which you name in the commit message.

- For a script of data logic, `xpd equiv <script.py> --since <commit>` prints
  `{"equivalent": true}`.
- `xpd check` reports no new issue in the files you changed.
- Remove what the fix makes unnecessary: the old copy, the flag, the branch, the wait.

Commit the fix and run `xpd untangle gate <fix>`. xpd runs the checks and compares the findings
with the commit before the fix.

| The gate keeps the fix when | Otherwise |
| --- | --- |
| Each check passes that passed at the start | Exit 1, with the check and the end of its output |
| At least one finding of the fix is gone | Exit 1 |
| The count of findings is not higher | Exit 1, with the ids of the new findings |

After a failed gate, repair the fix, commit and gate again. After two failed gates, restore
the files with `git reset --hard <commit>`, the commit that `status` names, and run
`xpd untangle drop <fix> --reason "<why>"`. The run goes on with the next fix.

A fix with `changes_interface` is the user's to decide. Ask the user, and run
`xpd untangle approve <fix>` only after a yes. The gate refuses the fix until then. After a no,
run `xpd untangle drop <fix> --reason "the user said no"`.

## Finish

When every fix of the plan is kept or dropped, read the ledger again and plan the next round.
When no finding is left that is worth a fix, run `xpd untangle finish`. It compares the
findings with the commit of the start and ends the run when each finding that is new since
then has a reason. It prints how many findings were fixed, added and left without a reason.

`xpd untangle --since <commit>` gives the same comparison at any time, outside a run: `fixed`
lists the findings that are gone, and `"new": true` marks a finding that was added.

## Done

- `xpd untangle finish` exits 0.
- Each remaining finding that matters has its `id` and a reason in the journal.
- The checks pass as they did at the start, and `xpd equiv` prints `{"equivalent": true}` for
  each script of data logic.

## What each kind measures

- `copy`: a whole definition or block of at least 5 lines and 60 tokens that is written out
  token for token in two or more places. Comments and layout are not compared.
- `changes-together`: two files that changed in the same commits at least 5 times, and in at
  least half of the commits of the one that changed less. A commit of more than 25 files is
  left out. A pair of Python files is listed only when neither imports the other.
- `shared-state`: a write to a name of another module, or a function that writes a module
  level name through `global`.
- `volatile-import`: a module that imports a module with at least twice as many commits,
  counted from the first commit of the importer. A test and an `__init__.py` are left out.
- `import-cycle`: modules that import one another in a circle.
- `private-reach`: an import or a read of a name of another module that starts with `_`.
- `vague-module`, `pass-through`, `disabled-test`: the rules `python/vague-module-name`,
  `python/pass-through-function` and `python/test-skip` of `xpd check`.
- `unreferenced`: a function or class at the top of a module that no Python file mentions
  apart from its definition. Code in another language, or a caller outside the paths, can
  still use it, so check before you delete.
- `constant-parameter`: a parameter that no call passes, or that every call gives the same
  literal. `one-caller-parameter`: a parameter with a default that one call alone passes.
  Both are judged only for a function whose every use is a call that can be matched.
- `one-value-branch`: an `if` without `elif` that compares something with one string. Each
  value is one finding with every place that checks it, so a value that many places check
  comes first: it is one decision of the design, written out in each place. Tests are left out.
- `swallowed-exception`: an `except` that only passes or continues, or `contextlib.suppress`.
- `sleep-or-retry`: `time.sleep`, `asyncio.sleep`, a retry decorator, and in a test the rules
  `python/test-sleep` and `python/test-rerun`.

The `id` of a finding is made from its kind and from what it is about, such as the file, the
name or the value, and not from a line. It stays the same when code moves within a file, and
it changes when the file is renamed.

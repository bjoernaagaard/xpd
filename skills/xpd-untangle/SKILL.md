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

The refactor is a loop of five stages, and no stage grades its own work. The command finds, you
judge and design, the fix is proven by the project's own checks, and the command measures.

```text
record the commit the refactor starts from, and run the project's checks
repeat
  read the ledger            xpd untangle --format json
  judge each finding         fix it, or accept it with a reason
  redesign                   one fix above the findings it removes
  fix and prove, one fix at a time
  measure                    xpd untangle --since <commit>
until no finding is left that is worth a fix
```

## Read the ledger

Run `git rev-parse HEAD` first and keep the commit, because every later measurement compares
with it. Run the project's checks and note which ones pass.

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

Accept a finding that is right as it is, with its `id` and the reason in one sentence, so that
a later round does not judge it again. Every other finding goes under a fix.

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

A special case that the system still needs stays, as an accepted finding with its reason.

## Fix and prove

Make one fix at a time, each on the result of the one before. The behavior stays the same,
apart from a bug the fix removes, which you name.

- Run the project's checks after the fix. Each one passes that passed before.
- For a script of data logic, `xpd equiv <script.py> --since <commit>` prints
  `{"equivalent": true}`.
- `xpd check` reports no new issue in the files you changed.
- Remove what the fix makes unnecessary: the old copy, the flag, the branch, the wait.

## Measure

`xpd untangle --since <commit>` compares with the commit the refactor started from.

| In the JSON | Meaning | What to do |
| --- | --- | --- |
| `fixed` | The findings that are gone | Check that each finding the fix was for is among them |
| `"new": true` on a finding | The fix added it | Remove it, or accept it with a reason |
| `was` beside a `count` | The count at the commit | A count that rose names a link the fix added |

Repeat from the ledger until no finding is left that is worth a fix.

## Done

- Each remaining finding has its `id` and a one sentence reason why the code is right as it is.
- No finding is `new` since the starting commit, unless it has such a reason.
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

When the xpd MCP server is connected, the tool `untangle` takes `paths`, `kinds`, `limit` and
`since` and returns the same JSON.

## As a T3 Code workflow

`untangle.workflow.ts`, beside this file, runs the loop as a durable workflow of T3 Code, with
a separate agent for each stage. A run survives a restart, and a fix that fails its gate is
dropped with its reason while the run goes on.

```sh
ln -s "$PWD/untangle.workflow.ts" ~/.t3/workflows/untangle.ts
t3 workflow start untangle --key my-repo-1 --input '{"project":"/path/to/repo","scope":"src"}'
```

Turn on Settings, Labs, Workflows first, and give a `project` that is a project of T3 Code.

| Input | Default | Meaning |
| --- | --- | --- |
| `project` | Required | The folder of the project |
| `scope` | `.` | The path below it to untangle |
| `rounds` | 3 | The most rounds |
| `fixes` | 6 | The most fixes of a round |
| `deep` | false | Design on the strongest models, which costs several times more |
| `xpd` | `xpd` | The xpd to run |

The run takes its models from `~/.config/bstack/t3-routes.json`, the routing table of the
`setup-bstack` skill, and without that file from the short list at the top of the workflow.
No stage names a model, and the stages take turns through the table, so the agent that makes a
fix is another model than the one that designed it.

| Stage | Tier | Which target of the tier |
| --- | --- | --- |
| survey | B | The first |
| design | A at high effort, or S with `deep` | The next one each round |
| fix | A | The next one for each fix, starting after the designer of the round |
| a second try of design or fix | The same tier | The target after the one that failed |

| Stage | Who does it | What comes back |
| --- | --- | --- |
| survey | One agent | The project's check commands and their results |
| ledger | A script that runs `xpd untangle` | The findings, each with its id |
| design | One agent that reads the code behind the findings | Fixes in order, each with the ids it removes, and the accepted findings with their reasons |
| ask | You | A yes for a fix that changes what callers or users see |
| fix | One agent for each fix, one after the other | A commit with the proof it ran, or the reason it made no change |
| gate | A script | The checks, run again, and `xpd untangle --since` on the commit |
| measure | A script | The count of each kind at the start and at the end |

The gate keeps a fix when the checks pass as they did before, at least one finding the fix was
for is gone, and the count of findings is not higher. A fix that fails gets one repair in the
same thread and is then dropped. The stages repeat on the result until a round keeps no fix.

The run changes nothing in the checkout. Each agent works in a worktree of its own, and the run
ends with one local branch, `untangle/<run>/result`, that holds every fix it kept. Nothing is
pushed or merged. The result of the run lists the fixes, the findings that were fixed and
added, and each accepted finding with its reason.

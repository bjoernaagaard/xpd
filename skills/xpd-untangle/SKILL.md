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

## Steps

1. **Record where the refactor starts.** Run `git rev-parse HEAD` and keep the commit. Run the
   project's tests, and for a script of data logic keep it unchanged for `xpd equiv`.
2. **Read the ledger.** `xpd untangle --format json` gives `summary`, with the count of each
   kind, and `findings`, the strongest of each kind with `places` and `evidence`.
   `--kind <kind> --limit 0` lists every finding of one kind. `xpd rules outline <paths…>` gives the
   skeleton of the files a finding names, so read that before whole files.
3. **Judge each finding against the code.** A finding is evidence, and you decide what it
   means. Two files that change together may be a feature split across folders, a copy of a
   value, or a test beside its subject. A branch for one value may be a rule of the domain.
   Drop a finding that is right as it is, and say why in one line.
4. **Redesign before you fix.** Group the findings that have one cause, and write the fix above
   them. Work the fix out as if the system had known today's requirements from the start. One
   fix that removes several findings often looks like none of them: a registry in place of five
   lists, an interface in place of a branch for each provider. Do not fix special cases one by
   one. For each fix, check that pieces that change together end up together, that pieces that
   do not are not tied, and that every import points at code that changes less.
5. **Fix one group at a time.** After each group, run `xpd check` on the files you changed, the
   project's tests, and `xpd equiv <script.py> --since <commit>` for a script of data logic.
6. **Measure.** `xpd untangle --since <commit>` prints each count beside the count at the
   commit of step 1. A count that went up names a link the fix added. Repeat from step 2 until
   no finding is left that is worth a fix.

## Done

- Each remaining finding has a one line reason why the code is right as it is.
- No count is higher than at the commit of step 1.
- The tests pass as they did at the start, and `xpd equiv` prints `{"equivalent": true}` for
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

When the xpd MCP server is connected, the tool `untangle` takes `paths`, `kinds`, `limit` and
`since` and returns the same JSON.

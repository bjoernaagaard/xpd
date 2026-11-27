---
name: xpd-discover
description: Make a Python codebase discoverable by plain text search with xpd discover. Find names that several modules define, bare verb names, role named modules, names built with f-strings and error messages that start with a value, and gate each rename in a recorded ledger.
---

# Make a codebase discoverable with xpd

Coding agents find code by searching for strings. Use `xpd discover` to make each public name, error message and key resolve in one search.

| Group | Finding kinds |
| --- | --- |
| Names that one search does not find | `name-collision`, `generic-verb-name`, `vague-module-name`, `module-private-name` |
| Strings and dead ends that a search misses | `interpolated-key`, `error-message-prefix`, `deprecated-without-decorator` |
| Types that let a caller swap arguments | `id-without-newtype` |

The header counts the public names that one module alone defines, such as `184 of 200 public names defined once`. A rename that makes two modules define one name adds a `name-collision` finding, which fails the gate. Each module with a finding gets six checklist items about search words in docstrings, one spelling per concept, one concept per module, thin coordinators and stale names. Run `xpd discover --questions` to read them.

A name that a rigid convention repeats on purpose, such as `list_documents` in every source plugin, goes in `accepted` with that reason. Rename everything else.

## Recorded run

1. **Start**: run `xpd discover start . --check "<project-check>"` from a clean commit.
2. **List**: run `xpd discover . --format json` to read finding IDs and checklist item IDs.
3. **Plan**: write `plan.json` with ordered `fixes` (naming finding IDs, with `"changes_interface": true` for a rename that other modules import, or `"goal": "design"` with `questions`), `accepted` findings with reasons, and `answered` checklist verdicts (`pass`, `na`, or `accepted`) with reasons. Run `xpd discover plan plan.json`.
4. **Approve**: run `xpd discover approve <fix-id>` after the user agrees to a rename that changes a public interface.
5. **Gate**: rename every definition and call in one commit and run `xpd discover gate <fix-id>`.
6. **Drop**: restore the run's current commit and run `xpd discover drop <fix-id> --reason "<why>"` if a fix is abandoned.
7. **Status and finish**: run `xpd discover status` to see the next stage, and `xpd discover finish` once every open fix, new finding, and checklist item is resolved.

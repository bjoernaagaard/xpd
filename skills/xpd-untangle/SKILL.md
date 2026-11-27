---
name: xpd-untangle
description: Refactor codebase structure, coupling, and dead weight with xpd untangle. Use source and Git history to eliminate copies, needless dependencies, and surplus code in a recorded ledger.
---

# Untangle codebase structure with xpd

Use `xpd untangle` to put pieces that change together in the same module, sever needless links, and remove dead weight. It reads source files and Git history without executing code.

| Group | Finding kinds |
| --- | --- |
| Hidden links | `copy`, `near-copy`, `changes-together`, `shared-state` |
| Needless links | `volatile-import`, `import-cycle`, `private-reach`, `vague-module` |
| Surplus | `pass-through`, `default-argument`, `unreferenced`, `constant-parameter` |
| Special cases | `one-value-branch`, `one-caller-parameter` |

Each module with a finding gets six structural checklist items. Run `xpd untangle --questions` to inspect the catalog. Supported fix goals are `shorter` (requires a lower line count) and `design` (requires editing a file owning a cited checklist item). For a data script refactor, prove behavior did not change with `xpd equiv` and note the result in the commit.

## Recorded run

1. **Start**: run `xpd untangle start . --check "<project-check>"` from a clean commit.
2. **List**: run `xpd untangle . --format json` to read stable finding IDs and checklist item IDs.
3. **Plan**: write `plan.json` with ordered `fixes` (naming finding IDs or a `shorter` or `design` goal), `accepted` findings with reasons, and `answered` checklist verdicts (`pass`, `na`, or `accepted`) with reasons. Run `xpd untangle plan plan.json`.
4. **Approve**: run `xpd untangle approve <fix-id>` after user approval for a fix that changes public interfaces.
5. **Gate**: commit one fix and run `xpd untangle gate <fix-id>`.
6. **Drop**: restore the run's current commit and run `xpd untangle drop <fix-id> --reason "<why>"` if a fix is abandoned.
7. **Status and finish**: run `xpd untangle status` to see the next stage, and `xpd untangle finish` once every open fix, new finding, and checklist item is resolved.

---
name: xpd-untangle
description: Refactor codebase structure, coupling, and dead weight with xpd untangle. Use source and Git history to map Python or Rust modules, apply mechanical moves or facade inlines, and eliminate copies, needless links, and surplus code in a recorded ledger.
---

# Untangle codebase structure with xpd

Use `xpd untangle` to put pieces that change together in the same module, sever needless links, and remove dead weight using source files and Git history without executing code.

| Group | Finding kinds |
| --- | --- |
| Hidden links | `copy`, `near-copy`, `changes-together`, `shared-state` |
| Needless links | `volatile-import`, `import-cycle`, `private-reach`, `vague-module`, `mixed-responsibilities` |
| Surplus | `pass-through`, `default-argument`, `unreferenced`, `constant-parameter`, `orphaned-module`, `facade-module`, `single-use-module` |
| Special cases | `one-value-branch`, `one-caller-parameter` |

Each module with a finding gets six structural checklist items (`xpd untangle --questions`). Use `xpd untangle <path> --map [--format json]` (or MCP `show_map`) to inspect Python or Rust coupling graphs, cycles, and orphans. For mechanical Python cleanups, preview diffs with `xpd untangle --move <from> <to>` or `xpd untangle --inline-facade <file>` and pass `--apply` to write `libcst`-validated rewrites. Supported fix goals are `shorter` (lower line count) and `design` (edits a file owning a cited checklist item). Prove data script refactors preserve output with `xpd equiv`.

## Recorded run

1. **Start**: run `xpd untangle start . --check "<project-check>"` from a clean commit.
2. **List**: run `xpd untangle . --format json` (or MCP `show_ledger`) to read stable finding IDs and checklist items.
3. **Plan**: write `plan.json` with ordered `fixes`, `accepted` findings with reasons, and `answered` checklist verdicts (`pass`, `na`, or `accepted`) with reasons. Run `xpd untangle plan plan.json`.
4. **Approve**: run `xpd untangle approve <fix-id>` after user approval for a fix that changes public interfaces.
5. **Gate**: commit one fix and run `xpd untangle gate <fix-id>`.
6. **Drop**: restore the run's current commit and run `xpd untangle drop <fix-id> --reason "<why>"` if abandoned.
7. **Status and finish**: check next stage with `xpd untangle status`, and close with `xpd untangle finish` once every fix, new finding, and checklist item is settled.

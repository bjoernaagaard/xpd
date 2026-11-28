Use `xpd lsp` as the language server of an editor, so the editor shows the issues of
`xpd check` as you type, with rule help on hover, fixes as code actions, and workspace-level
structural `HINT` diagnostics from [`xpd untangle`](../untangle/) on open and save
(`untangle/import-cycle`, `untangle/orphaned-module`, `untangle/facade-module`,
`untangle/single-use-module`, `untangle/mixed-responsibilities`, and `untangle/shared-state`).
It speaks the Language Server Protocol on stdin and stdout, and lints the text the editor holds,
saved or not. It reads one setting, `unsafeFixes`, from the editor. The
[editor guide](../../guides/editors/) shows the setup.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The editor sent `shutdown` and then `exit` |
| 1 | The editor went away without them, or never sent `initialize` |

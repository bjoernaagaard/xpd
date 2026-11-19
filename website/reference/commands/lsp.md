Use `xpd lsp` as the language server of an editor, so the editor shows the issues of
`xpd check` as you type, with rule help on hover and fixes as code actions. It speaks the
Language Server Protocol on stdin and stdout, and lints the text the editor holds, saved or not.
It reads one setting, `unsafeFixes`, from the editor. The
[language server guide](https://bjoernaagaard.github.io/xpd/lsp.html) shows the setup.

## Exit status

| Exit code | Meaning |
| --- | --- |
| 0 | The editor sent `shutdown` and then `exit` |
| 1 | The editor went away without them, or never sent `initialize` |

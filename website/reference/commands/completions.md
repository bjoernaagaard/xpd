Use `xpd completions` to let your shell complete xpd's commands and flags. It prints the
completion script of a shell, for every command and flag of the installed xpd.

## Examples

```sh
xpd completions bash > ~/.local/share/bash-completion/completions/xpd
xpd completions zsh > "${fpath[1]}/_xpd"
```

## Exit status

It exits 0, and 2 for a shell it does not know.

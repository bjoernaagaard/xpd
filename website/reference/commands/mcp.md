Use `xpd mcp` to give a coding agent xpd's tools through the Model Context Protocol. It starts
the MCP server, which is Python and installs with the xpd package, and the arguments after `mcp`
go to the server. The [MCP server reference](../../mcp/) lists its tools, resources and prompts.

## Examples

```sh
claude mcp add xpd -- uvx xpd mcp
uvx xpd mcp --transport http --port 8000
```

A binary built with cargo has no Python beside it, so its `xpd mcp` prints the command to use
instead.

```console
$ xpd mcp
xpd: xpd mcp needs xpd installed as a Python package. Run it as: uvx xpd mcp
```

## Exit status

The server's exit code passes through. `xpd mcp` itself exits 2 when the server cannot start,
as when xpd is not installed as a Python package.

# Pin the packages of the MCP server narrowly

Status. Accepted.

The MCP server, `xpd mcp`, needs FastMCP with its apps and tasks extras. FastMCP stays within one minor release, because a new minor release can change what the server registers. prefab-ui draws the app views, and it is pinned to one version because its releases break often.

Evidence. See `dependencies` in `pyproject.toml`.

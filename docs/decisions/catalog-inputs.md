# Stamp rule inputs before reading the catalog

The MCP server caches the result of running `xpd rules` for a project. It reads the input paths, modification times and sizes before starting the command and saves that stamp with the result. If a file changes during the command and the stamp changes, the next request reloads the catalog. An edit that preserves all recorded fields is outside this check.

See `python/xpd/mcp/catalog.py`, method `Catalogs.load`, at revision 8f1403b.

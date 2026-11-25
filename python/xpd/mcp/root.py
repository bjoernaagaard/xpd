"""The project root. Tools read files, `fix` writes them and `equiv` runs scripts, so every path
a client passes must resolve inside it."""

from pathlib import Path
from typing import NamedTuple
from urllib.parse import urlparse
from urllib.request import url2pathname

from fastmcp import Context
from fastmcp.exceptions import ToolError
from mcp.types import (
    ClientCapabilities,
    InputRequiredResult,
    ListRootsRequest,
    ListRootsResult,
    RootsCapability,
)
from mcp_types.version import MODERN_PROTOCOL_VERSIONS

# The key of the client's roots in a round of the modern protocol.
ROOTS = "project_roots"
NO_ROOTS = (
    "xpd does not know the project root, because the client names no folder as a root. "
    "Start the server with --root or XPD_MCP_ROOT."
)


def uses_handshake(ctx: Context) -> bool:
    """Whether the client can be asked mid-call. The 2026-07-28 protocol and background tasks
    ask by returning an InputRequiredResult instead."""
    request = ctx.request_context
    return (
        not ctx.is_background_task
        and request is not None
        and request.protocol_version not in MODERN_PROTOCOL_VERSIONS
    )


class Root(NamedTuple):
    path: Path

    @classmethod
    def at(cls, path: Path) -> "Root":
        return cls(path.resolve(strict=True))

    def inside(self, raw: str) -> str:
        """`raw` relative to the root, as the binary should get it so its output names files the
        way the client does. A symlink counts where it points."""
        resolved = (self.path / raw).resolve()
        if not resolved.is_relative_to(self.path):
            msg = f"{raw} is outside the project root {self.path}. Pass a path inside it."
            raise ToolError(msg)
        return resolved.relative_to(self.path).as_posix()

    def all_inside(self, paths: list[str]) -> list[str]:
        return [self.inside(raw) for raw in paths]


class RootsRequiredError(Exception):
    def __init__(self, request: InputRequiredResult) -> None:
        super().__init__()
        self.request = request


class Project(NamedTuple):
    """Where the root of a call comes from: the server's settings, or else the client's roots,
    which is how a server that starts outside the project learns where the project is."""

    fixed: Root | None

    def roots_request(self, ctx: Context) -> dict[str, ListRootsRequest]:
        """What a tool adds to another question it returns, so the next round still has the
        roots."""
        asks = self.fixed is None and not uses_handshake(ctx)
        return {ROOTS: ListRootsRequest()} if asks else {}

    async def required_root(self, ctx: Context) -> Root:
        result = await self.root(ctx)
        if isinstance(result, InputRequiredResult):
            raise RootsRequiredError(result)
        return result

    async def root(self, ctx: Context) -> Root | InputRequiredResult:
        """The root, or the request that asks the client for its roots on this protocol."""
        if self.fixed is not None:
            return self.fixed
        if uses_handshake(ctx):
            if not ctx.session.check_client_capability(ClientCapabilities(roots=RootsCapability())):
                raise ToolError(NO_ROOTS)
            listed = await ctx.session.list_roots()
        else:
            responses = ctx.input_responses
            if responses is None or ROOTS not in responses:
                return InputRequiredResult(
                    result_type="input_required", input_requests=self.roots_request(ctx)
                )
            listed = responses[ROOTS]
        if not isinstance(listed, ListRootsResult):
            raise ToolError(NO_ROOTS)
        for root in listed.roots:
            if folder := local_folder(str(root.uri)):
                return Root.at(folder)
        raise ToolError(NO_ROOTS)


def local_folder(uri: str) -> Path | None:
    """The folder that a `file:` URI names on this machine, if it is one."""
    parsed = urlparse(uri)
    if parsed.scheme != "file" or parsed.netloc not in {"", "localhost"}:
        return None
    path = Path(url2pathname(parsed.path.replace("%3A", ":").replace("%3a", ":")))
    return path if path.is_dir() else None

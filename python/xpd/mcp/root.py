"""The project root. Tools read files, `fix` writes them and `equiv` runs scripts, so every path
a client passes must resolve inside it."""

from dataclasses import dataclass
from pathlib import Path

from fastmcp.exceptions import ToolError


@dataclass(frozen=True)
class Root:
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

"""Runs the xpd binary, which owns every rule and result, and parses what it prints."""

import asyncio
import os
import shutil
import subprocess
import sys
import sysconfig
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Generic, TypeVar

from fastmcp.exceptions import ToolError
from pydantic import TypeAdapter

from xpd.mcp.models import CheckReport, Equivalence, GraphReport, Profile, Rule, SkillSource

# Exit codes that carry a result. 1 means the result holds an error, such as a failing check.
FINDINGS = frozenset({0, 1})
SUCCESS = frozenset({0})

# The type parameter syntax of Python 3.12 is out, because the wheel supports 3.11.
T = TypeVar("T")


@dataclass(frozen=True)
class Command(Generic[T]):
    """One xpd command: its leading arguments, the shape of its JSON and its result codes."""

    args: tuple[str, ...]
    output: TypeAdapter[T]
    results: frozenset[int]


CHECK = Command(("check", "--format", "json"), TypeAdapter(CheckReport), FINDINGS)
GRAPH = Command(("graph", "--format", "json"), TypeAdapter(GraphReport), FINDINGS)
# profile exits 1 when it cannot read the file, so only 0 carries a profile.
PROFILE = Command(("profile",), TypeAdapter(Profile), SUCCESS)
EQUIV = Command(("equiv",), TypeAdapter(Equivalence), FINDINGS)
RULES = Command(("rules", "--format", "json"), TypeAdapter(list[Rule]), SUCCESS)
SKILLS = Command(("skill", "--json"), TypeAdapter(list[SkillSource]), SUCCESS)


def argv(paths: Sequence[str], flags: Sequence[str]) -> tuple[str, ...]:
    """The arguments after the command. A client names the paths, so they follow `--`, which
    keeps a path such as `--fix` from being read as a flag."""
    return (*flags, "--", *paths)


def find_binary() -> Path:
    """`XPD_BIN`, which `xpd mcp` sets, then the scripts folder of this Python, then `PATH`."""
    name = "xpd.exe" if sys.platform == "win32" else "xpd"
    candidates = [
        os.environ.get("XPD_BIN"),
        str(Path(sysconfig.get_path("scripts")) / name),
        shutil.which("xpd"),
    ]
    for candidate in candidates:
        if candidate and Path(candidate).is_file():
            return Path(candidate)
    msg = "cannot find the xpd binary. Set XPD_BIN, or install xpd in this environment."
    raise RuntimeError(msg)


@dataclass(frozen=True)
class Xpd:
    binary: Path

    async def run(self, command: Command[T], cwd: Path, paths: Sequence[str], *flags: str) -> T:
        """Runs in `cwd`, the project root, so the binary finds the project's venv as a user's
        run does."""
        args = argv(paths, flags)
        process = await asyncio.create_subprocess_exec(
            self.binary,
            *command.args,
            *args,
            cwd=cwd,
            stdin=asyncio.subprocess.DEVNULL,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await process.communicate()
        return self._parse(command, args, process.returncode, stdout, stderr)

    def run_now(self, command: Command[T]) -> T:
        """Runs a command that answers at once, for what the server needs before it starts."""
        done = subprocess.run(  # noqa: S603  the binary and its arguments come from this module
            [self.binary, *command.args],
            stdin=subprocess.DEVNULL,
            capture_output=True,
            check=False,
        )
        return self._parse(command, (), done.returncode, done.stdout, done.stderr)

    def version(self) -> str:
        done = subprocess.run(  # noqa: S603  the binary comes from find_binary
            [self.binary, "--version"], capture_output=True, text=True, check=True
        )
        # clap prints `xpd 0.12.0`.
        return done.stdout.split()[-1]

    @staticmethod
    def _parse(
        command: Command[T],
        args: tuple[str, ...],
        code: int | None,
        stdout: bytes,
        stderr: bytes,
    ) -> T:
        if code not in command.results:
            # equiv reports a failing script in its JSON on stdout, the others on stderr.
            reason = (stderr or stdout).decode(errors="replace").strip()
            shown = " ".join([*command.args[:1], *args])
            msg = f"xpd {shown} exited with {code}: {reason}"
            raise ToolError(msg)
        return command.output.validate_json(stdout)

"""Runs the xpd binary, which owns every rule and result, and parses what it prints."""

import asyncio
import contextlib
import json
import os
import shutil
import subprocess
import sys
import sysconfig
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import NamedTuple

from fastmcp.exceptions import ToolError
from pydantic import TypeAdapter

from xpd.mcp.models import (
    CheckReport,
    CouplingMapReport,
    DiscoveryEntry,
    GateReport,
    GraphReport,
    LedgerWorkflowName,
    Profile,
    Rule,
    SkillSource,
)

# Exit codes that carry a result. 1 means the result holds an error, such as a failing check.
FINDINGS = frozenset({0, 1})
SUCCESS = frozenset({0})


class Command[T](NamedTuple):
    """One xpd command: its leading arguments, the shape of its JSON and its result codes."""

    args: tuple[str, ...]
    decode: Callable[[bytes], T]
    results: frozenset[int]


API = Command(("api", "--format", "json"), TypeAdapter(list[DiscoveryEntry]).validate_json, SUCCESS)
CHECK = Command(("check", "--format", "json"), TypeAdapter(CheckReport).validate_json, FINDINGS)
GRAPH = Command(("graph", "--format", "json"), TypeAdapter(GraphReport).validate_json, FINDINGS)
PROFILE = Command(("profile",), TypeAdapter(Profile).validate_json, SUCCESS)
GATES_START = Command(
    ("gates", "start", "--format", "json"), TypeAdapter(GateReport).validate_json, FINDINGS
)
GATES_RETRY = Command(
    ("gates", "retry", "--format", "json"), TypeAdapter(GateReport).validate_json, FINDINGS
)
RULES = Command(("rules", "--format", "json"), TypeAdapter(list[Rule]).validate_json, SUCCESS)
BUILTIN_RULES = Command(("rules", "--builtin", "--format", "json"), RULES.decode, SUCCESS)
SKILLS = Command(("skill", "--json"), TypeAdapter(list[SkillSource]).validate_json, SUCCESS)
UNTANGLE_MAP = Command(
    ("untangle", "--map", "--format", "json"),
    TypeAdapter(CouplingMapReport).validate_json,
    FINDINGS,
)


def ledger_status_command(workflow: LedgerWorkflowName) -> Command[dict[str, object]]:
    return Command((workflow, "status", "--format", "json"), json.loads, FINDINGS)


def ledger_survey_command(workflow: LedgerWorkflowName) -> Command[dict[str, object]]:
    return Command((workflow, "--format", "json"), json.loads, FINDINGS)


def json_error(stdout: bytes) -> bytes | None:
    """The error that equiv and refit report as JSON on stdout. The script's own prints and
    warnings go to stderr, so stderr alone would hide the reason."""
    try:
        report = json.loads(stdout)
    except ValueError:
        return None
    error = report.get("error") if isinstance(report, dict) else None
    return error.encode() if isinstance(error, str) else None


def argv(paths: Sequence[str], flags: Sequence[str]) -> tuple[str, ...]:
    """The arguments after the command. A client names the paths, so they follow `--`, which
    keeps a path such as `--fix` from being read as a flag."""
    if not paths:
        return tuple(flags)
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
            return Path(candidate).absolute()
    msg = "cannot find the xpd binary. Set XPD_BIN, or install xpd in this environment."
    raise RuntimeError(msg)


class Xpd(NamedTuple):
    binary: Path

    async def run[T](
        self,
        command: Command[T],
        cwd: Path,
        paths: Sequence[str],
        *flags: str,
        project_root: Path | None = None,
        stdin: bytes | None = None,
    ) -> T:
        """Runs in `cwd`, the project root, so the binary finds the project's venv as a user's
        run does."""
        args = argv(paths, flags)
        env = {**os.environ, "XPD_PROJECT_ROOT": str(project_root)} if project_root else None
        process = await asyncio.create_subprocess_exec(
            self.binary,
            *command.args,
            *args,
            cwd=cwd,
            env=env,
            stdin=asyncio.subprocess.DEVNULL if stdin is None else asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        try:
            stdout, stderr = await process.communicate(stdin)
        except asyncio.CancelledError:
            if process.returncode is None:
                with contextlib.suppress(ProcessLookupError):
                    process.terminate()
                try:
                    await asyncio.wait_for(process.wait(), timeout=2.0)
                except TimeoutError:
                    with contextlib.suppress(ProcessLookupError):
                        process.kill()
                    await process.wait()
            raise
        return self._parse(command, args, process.returncode, stdout, stderr)

    def run_now[T](self, command: Command[T]) -> T:
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
    def _parse[T](
        command: Command[T],
        args: tuple[str, ...],
        code: int | None,
        stdout: bytes,
        stderr: bytes,
    ) -> T:
        if code not in command.results:
            reason = (json_error(stdout) or stderr or stdout).decode(errors="replace").strip()
            shown = " ".join([*command.args[:1], *args])
            msg = f"xpd {shown} exited with {code}: {reason}"
            raise ToolError(msg)
        return command.decode(stdout)

"""Discover Manim animation recipes and render or retrieve animations through local MCP."""

import asyncio
import base64
import os
import re
import stat
from contextlib import ExitStack
from pathlib import Path
from typing import Literal

from fastmcp import Context, FastMCP
from fastmcp.exceptions import ToolError
from fastmcp.tools import ToolResult
from fastmcp.utilities.types import Image
from mcp.types import BlobResourceContents, EmbeddedResource, InputRequiredResult
from pydantic import BaseModel, TypeAdapter, computed_field

from xpd.mcp.base import READS, RUNS_CODE, rooted
from xpd.mcp.binary import SUCCESS, Command, Xpd
from xpd.mcp.models import Output
from xpd.mcp.names import ToolName
from xpd.mcp.root import Project, Root, RootsRequiredError

Profile = Literal["preview", "final", "still"]
Artifact = Literal["video", "still", "contact_sheet", "source", "manifest", "log"]

MIB = 1024 * 1024
TEXT_LIMIT = 128 * 1024
ARTIFACTS: dict[Artifact, tuple[str, str, int]] = {
    "video": ("video.mp4", "video/mp4", 64 * MIB),
    "still": ("still.png", "image/png", 8 * MIB),
    "contact_sheet": ("contact-sheet.png", "image/png", 8 * MIB),
    "source": ("source.zip", "application/zip", 34 * MIB),
    "manifest": ("manifest.json", "application/json", 256 * 1024),
    "log": ("render.log", "text/plain", TEXT_LIMIT),
}
JOB_ID = re.compile(r"[a-f0-9]{32}")


class AnimationStatus(Output):
    ready: bool
    version: str
    image: str
    reason: str | None = None


class AnimationRender(Output):
    id: str
    profile: Profile
    scene: str
    entry: str
    files: list[str]
    renderer: str
    duration_seconds: float | None
    width: int
    height: int
    fps: float | None
    video: str | None
    still: str | None
    contact_sheet: str | None
    source: str
    manifest: str
    log: str
    source_dir: str
    elapsed_seconds: float
    cached: bool

    @computed_field
    @property
    def resources(self) -> dict[str, str]:
        return {
            artifact: artifact_uri(self.id, filename)
            for artifact, (filename, _, _) in ARTIFACTS.items()
            if getattr(self, artifact)
        }


class AnimationSource(Output):
    file: str
    text: str


class InlineScene(BaseModel):
    entry: str
    files: dict[str, str]
    from_job: str | None = None


GUIDE = Command(("animate", "guide"), bytes.decode, SUCCESS)
STATUS = Command(
    ("animate", "doctor", "--json"), TypeAdapter(AnimationStatus).validate_json, SUCCESS
)
RENDER = Command(
    ("animate", "render", "--json"), TypeAdapter(AnimationRender).validate_json, SUCCESS
)


def artifact_uri(job_id: str, filename: str) -> str:
    return f"xpd://animation/{job_id}/{filename}"


def source_parts(file: str) -> tuple[str, ...]:
    parts = tuple(file.split("/"))
    if any(part in {"", ".", ".."} or "\\" in part or "\x00" in part for part in parts):
        msg = "Pass a relative path within the saved scene's source directory."
        raise ToolError(msg)
    return parts


def read_job_file(root: Root, job_id: str, parts: tuple[str, ...], limit: int) -> bytes:
    if not JOB_ID.fullmatch(job_id):
        msg = "Pass the 32-character render ID returned by animate_render."
        raise ToolError(msg)
    if os.open not in os.supports_dir_fd:
        msg = "Reading Manim render artifacts requires a supported local renderer host."
        raise ToolError(msg)

    directories = (".xpd-animate", "jobs", job_id, *parts[:-1])
    name = parts[-1]
    try:
        with ExitStack() as opened:
            directory_flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW
            parent = os.open(root.path, directory_flags)
            opened.callback(os.close, parent)
            for directory in directories:
                parent = os.open(directory, directory_flags, dir_fd=parent)
                opened.callback(os.close, parent)
            fd = os.open(name, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=parent)
            with os.fdopen(fd, "rb") as stream:
                info = os.fstat(stream.fileno())
                if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1:
                    msg = f"{name} is not a regular render artifact."
                    raise ToolError(msg)
                data = stream.read(limit + 1) if info.st_size <= limit else b""
                if info.st_size > limit or len(data) > limit:
                    relative = "/".join((*directories, name))
                    msg = (
                        f"{name} exceeds the {limit:,}-byte MCP transfer limit. "
                        f"Open {relative} locally."
                    )
                    raise ToolError(msg)
                return data
    except OSError as error:
        msg = f"Render {job_id} has no readable {name} in this project."
        raise ToolError(msg) from error


async def read_artifact(root: Root, job_id: str, artifact: Artifact) -> bytes:
    filename, _, limit = ARTIFACTS[artifact]
    return await asyncio.to_thread(read_job_file, root, job_id, (filename,), limit)


async def resource_root(project: Project, ctx: Context) -> Root:
    try:
        return await project.required_root(ctx)
    except RootsRequiredError as error:
        msg = "Use animate_artifact so the client can provide its project root."
        raise ToolError(msg) from error


def register(mcp: FastMCP, xpd: Xpd, project: Project, *, local: bool) -> None:
    @mcp.tool(name=ToolName.ANIMATE_GUIDE, tags={"animation"}, annotations=READS)
    async def animate_guide(recipe: str | None = None) -> str:
        """Read the offline Manim workflow and available recipes. Start here when asked to
        turn a concept into an animation. Pick a nearby recipe, or write a custom Manim scene.

        Args:
            recipe: Recipe name from the guide. Omit it to see the workflow and recipe names.
        """
        return await xpd.run(GUIDE, Path.cwd(), (recipe,) if recipe else ())

    @mcp.tool(name=ToolName.ANIMATE_STATUS, tags={"animation"}, annotations=READS)
    async def animate_status() -> AnimationStatus:
        """Report whether the pinned Manim renderer is ready and what to do if setup is needed."""
        return await xpd.run(STATUS, Path.cwd(), ())

    if local:
        register_local_tools(mcp, xpd, project)
        register_resources(mcp, project)


def register_local_tools(mcp: FastMCP, xpd: Xpd, project: Project) -> None:
    @mcp.tool(name=ToolName.ANIMATE_RENDER, tags={"animation"}, annotations=RUNS_CODE, task=True)
    @rooted
    async def animate_render(
        ctx: Context,
        scene: str,
        source: str | InlineScene,
        *,
        profile: Profile = "preview",
        assets: list[str] | None = None,
    ) -> AnimationRender:
        """Render a Manim scene in the local isolated renderer. Start with preview, inspect
        its contact sheet with animate_artifact, then render final. Use animate_source to
        recover editable source in a later session.

        Args:
            scene: Manim Scene class name.
            source: Existing scene path inside the project, or inline source with an entry
                path and a map of bundle-relative paths to UTF-8 source. To revise, set
                from_job to a saved render ID and include only changed files in the map.
            profile: Preview for iteration, final for delivery, or still for the last frame.
            assets: Existing helper or asset paths relative to the project root.
        """
        root = await project.required_root(ctx)
        flags = [f"--profile={profile}"]
        flags.extend(f"--asset={asset}" for asset in root.all_inside(assets or []))
        payload = None
        if isinstance(source, str):
            path = root.inside(source)
            if path == "-":
                msg = "Use the inline source object to send scene source."
                raise ToolError(msg)
        else:
            path = "-"
            payload = source.model_dump_json().encode()
        await ctx.report_progress(0, 1, f"Rendering {profile} animation")
        result = await xpd.run(RENDER, root.path, [path, scene], *flags, stdin=payload)
        await ctx.report_progress(1, 1, f"Render {result.id} finished")
        return result

    @mcp.tool(name=ToolName.ANIMATE_SOURCE, tags={"animation"}, annotations=READS)
    @rooted
    async def animate_source(ctx: Context, job_id: str, file: str | None = None) -> AnimationSource:
        """Read saved Manim source as text to revise a scene without access to the local files.

        Args:
            job_id: ID returned by animate_render.
            file: Path listed in the saved manifest's files. Defaults to the main entry.
        """
        root = await project.required_root(ctx)
        if file is None:
            manifest = await read_artifact(root, job_id, "manifest")
            file = AnimationRender.model_validate_json(manifest).entry
        parts = ("source", *source_parts(file))
        data = await asyncio.to_thread(read_job_file, root, job_id, parts, TEXT_LIMIT)
        try:
            return AnimationSource(file=file, text=data.decode())
        except UnicodeDecodeError as error:
            msg = f"{file} is binary. Use animate_artifact to download the source archive."
            raise ToolError(msg) from error

    @mcp.tool(name=ToolName.ANIMATE_ARTIFACT, tags={"animation"}, annotations=READS)
    @rooted
    async def animate_artifact(
        ctx: Context, job_id: str, artifact: Artifact = "contact_sheet"
    ) -> ToolResult | InputRequiredResult:
        """Inspect a contact sheet or still, download an MP4 or source archive, or read the
        manifest or render log. Media is transferred only when explicitly requested.

        Args:
            job_id: ID returned by animate_render.
            artifact: Which file to retrieve. Defaults to the contact sheet.
        """
        root = await project.required_root(ctx)
        data = await read_artifact(root, job_id, artifact)
        filename, mime, _ = ARTIFACTS[artifact]
        if mime == "image/png":
            return ToolResult(content=[Image(data=data, format="png").to_image_content()])
        if artifact in {"manifest", "log"}:
            return ToolResult(content=data.decode(errors="replace"))
        return ToolResult(
            content=[
                EmbeddedResource(
                    type="resource",
                    resource=BlobResourceContents(
                        uri=artifact_uri(job_id, filename),
                        mime_type=mime,
                        blob=base64.b64encode(data).decode(),
                    ),
                )
            ]
        )


def register_resources(mcp: FastMCP, project: Project) -> None:
    def register_artifact(artifact: Artifact, filename: str, mime: str) -> None:
        @mcp.resource(f"xpd://animation/{{job_id}}/{filename}", name=filename, mime_type=mime)
        async def animation_file(job_id: str, ctx: Context) -> str | bytes:
            data = await read_artifact(await resource_root(project, ctx), job_id, artifact)
            return data.decode(errors="replace") if artifact in {"manifest", "log"} else data

    for artifact, (filename, mime, _) in ARTIFACTS.items():
        register_artifact(artifact, filename, mime)

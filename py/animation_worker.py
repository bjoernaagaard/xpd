"""Render in container scratch and return a bounded archive of known media files."""

import os
import re
import shutil
import stat
import subprocess
import sys
import tarfile
from pathlib import Path

SEGMENT = re.compile(r"[0-9a-f]{64}\.mp4")


def add_regular(archive: tarfile.TarFile, path: Path, name: str, limit: int) -> int:
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    with os.fdopen(os.open(path, flags), "rb") as stream:
        metadata = os.fstat(stream.fileno())
        if not stat.S_ISREG(metadata.st_mode) or not 0 < metadata.st_size <= limit:
            msg = f"Expected a regular file of at most {limit} bytes: {path.name}"
            raise ValueError(msg)
        member = tarfile.TarInfo(name)
        member.size = metadata.st_size
        archive.addfile(member, stream)
    return metadata.st_size


def add_segments(archive: tarfile.TarFile, folder: Path, byte_limit: int, file_limit: int) -> None:
    if folder.resolve() != folder or not folder.is_dir():
        return
    remaining = byte_limit
    for path in sorted(folder.iterdir())[:file_limit]:
        if SEGMENT.fullmatch(path.name):
            try:
                remaining -= add_regular(archive, path, f"segments/{path.name}", remaining)
            except (OSError, ValueError):
                continue


def main() -> int:
    entry, scene, size, fps, media_format, media_limit, segment_limit, file_limit = sys.argv[1:]
    scratch = Path("/tmp")  # noqa: S108
    source = shutil.copytree("/source", scratch / "scene")
    segments = scratch / "segments"
    shutil.copytree("/seed", segments / scene)

    rendered = subprocess.run(  # noqa: S603
        [
            sys.executable,
            "-P",
            "-m",
            "manim",
            "render",
            "--silent",
            "--config_file",
            "/xpd-manim.cfg",
            "--renderer",
            "cairo",
            "--format",
            media_format,
            "--progress_bar",
            "none",
            "--resolution",
            size,
            "--fps",
            fps,
            "--seed",
            "0",
            "--output_file",
            "artifact",
            "--",
            str(source / entry),
            scene,
        ],
        cwd=source,
        stdin=subprocess.DEVNULL,
        stdout=sys.stderr,
        check=False,
    )
    if rendered.returncode:
        return rendered.returncode

    output = scratch / "output" / scene
    if output.resolve() != output or not output.is_dir():
        msg = f"Manim did not produce an artifact for {scene} in {entry}."
        raise ValueError(msg)
    name = f"artifact.{media_format}"
    with tarfile.open(mode="w|", fileobj=sys.stdout.buffer, format=tarfile.USTAR_FORMAT) as archive:
        add_regular(archive, output / name, name, int(media_limit))
        add_segments(archive, segments / scene, int(segment_limit), int(file_limit))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError) as error:
        print(error, file=sys.stderr)
        sys.exit(2)

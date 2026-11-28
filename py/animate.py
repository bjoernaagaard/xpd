"""Run a pinned Manim worker and publish its verified artifacts."""

import argparse
import contextlib
import fcntl
import hashlib
import io
import json
import os
import re
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
from configparser import ConfigParser
from configparser import Error as ConfigError
from pathlib import Path
from typing import NamedTuple, TypedDict
from uuid import uuid4
from zipfile import ZIP_DEFLATED, BadZipFile, ZipFile, ZipInfo

VERSION = "0.22.1"
IMAGE = (
    "docker.io/manimcommunity/manim@"
    "sha256:a415f19f65a09fb481cbcb167adaecff86f4b782543e682362db3dea337dd0d9"
)
PODMAN = ("podman", "--remote=false")
STATE = ".xpd-animate"
INPUT_LIMIT = 32 * 1024 * 1024
INLINE_LIMIT = 1024 * 1024
VIDEO_LIMIT = 64 * 1024 * 1024
PNG_LIMIT = 8 * 1024 * 1024
SEGMENT_LIMIT = 64 * 1024 * 1024
EXPORT_LIMIT = VIDEO_LIMIT + SEGMENT_LIMIT + 1024 * 1024
LOG_LIMIT = 128 * 1024
MAX_FILES = 128
SEGMENT = re.compile(r"[0-9a-f]{64}\.mp4")
JOB_ID = re.compile(r"[0-9a-f]{32}")
CACHE_SCHEMA = "4"
CONFIG = """[CLI]
input_file =
media_dir = /tmp/media
video_dir = /tmp/output/{scene_name}
images_dir = /tmp/output/{scene_name}
partial_movie_dir = /tmp/segments/{scene_name}
sections_dir = /tmp/output/{scene_name}/sections
tex_dir = /tmp/media/Tex
text_dir = /tmp/media/texts
log_dir = /tmp/media/logs
max_inflight_encoders = 2
write_all = False
dry_run = False
preview = False
live_preview = False
show_in_file_browser = False
save_sections = False
"""
PROFILES = {
    "draft": ("-ql", "mp4"),
    "preview": ("-qh", "mp4"),
    "final": ("-qk", "mp4"),
    "still": ("-qm", "png"),
}


class AnimationError(Exception):
    """An actionable failure from the animation workflow."""


class MediaStats(TypedDict):
    width: int
    height: int
    fps: int | None
    duration_seconds: float | None


class RenderReport(MediaStats):
    id: str
    scene: str
    entry: str
    files: list[str]
    profile: str
    renderer: str
    video: str | None
    still: str | None
    contact_sheet: str | None
    source: str
    source_dir: str
    manifest: str
    log: str
    elapsed_seconds: float
    cached: bool
    fingerprint: str
    artifact_hashes: dict[str, str]


class Scene(NamedTuple):
    entry: Path
    name: str
    files: dict[Path, bytes]

    def fingerprint(self, profile: str, *, normalized_entry: bytes | None = None) -> str:
        digest = hashlib.sha256()
        for metadata in (CACHE_SCHEMA, IMAGE, CONFIG, profile, str(self.entry), self.name):
            digest.update(metadata.encode() + b"\0")
        for path, contents in sorted(self.files.items()):
            data = (
                normalized_entry
                if path == self.entry and normalized_entry is not None
                else contents
            )
            digest.update(path.as_posix().encode() + b"\0" + hashlib.sha256(data).digest())
        return digest.hexdigest()

    def segment_key(self, profile: str) -> str:
        digest = hashlib.sha256()
        for metadata in (CACHE_SCHEMA, IMAGE, CONFIG, profile, self.entry.as_posix(), self.name):
            digest.update(metadata.encode() + b"\0")
        return digest.hexdigest()


class Render(NamedTuple):
    scene: Scene
    profile: str
    job_id: str
    fingerprint: str
    started: float

    @property
    def path(self) -> Path:
        return Path(STATE) / "jobs" / self.job_id

    def elapsed(self) -> float:
        return round(time.monotonic() - self.started, 3)


def positive_path(value: str) -> Path:
    path = Path(value)
    if path.is_absolute() or ".." in path.parts or not path.parts or "\\" in value or "\0" in value:
        msg = f"Use a project-relative file path without '..': {value!r}"
        raise AnimationError(msg)
    return path


def read_regular(path: Path, limit: int) -> bytes:
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK
    with os.fdopen(os.open(path, flags), "rb") as stream:
        metadata = os.fstat(stream.fileno())
        if not stat.S_ISREG(metadata.st_mode) or metadata.st_size > limit:
            msg = f"Expected a regular file no larger than {limit} bytes: {path.name}"
            raise AnimationError(msg)
        data = stream.read(limit + 1)
    if len(data) > limit:
        msg = f"File grew beyond {limit} bytes: {path.name}"
        raise AnimationError(msg)
    return data


def saved_files(root: Path, job_id: str) -> dict[Path, bytes]:
    folder = root / STATE / "jobs" / job_id
    if not JOB_ID.fullmatch(job_id) or folder.resolve() != folder:
        msg = "Pass the ID of a saved render from this project."
        raise AnimationError(msg)
    archive = read_regular(folder / "source.zip", INPUT_LIMIT + 2 * INLINE_LIMIT)
    with ZipFile(io.BytesIO(archive)) as bundle:
        return archive_files(bundle)


def archive_files(bundle: ZipFile) -> dict[Path, bytes]:
    files = {}
    remaining = INPUT_LIMIT
    entries = bundle.infolist()
    if len(entries) > MAX_FILES:
        msg = f"A saved render can include at most {MAX_FILES} files."
        raise AnimationError(msg)
    for item in entries:
        path = positive_path(item.filename)
        if item.is_dir() or path in files or item.file_size > remaining:
            msg = f"Invalid or oversized file in saved source: {item.filename}"
            raise AnimationError(msg)
        with bundle.open(item) as content:
            data = content.read(remaining + 1)
        remaining -= len(data)
        if remaining < 0:
            msg = "Saved source exceeds the total scene input limit."
            raise AnimationError(msg)
        files[path] = data
    return files


def inline_files(root: Path) -> tuple[Path, dict[Path, bytes]]:
    raw = sys.stdin.buffer.read(INLINE_LIMIT + 1)
    if len(raw) > INLINE_LIMIT:
        msg = "Inline source bundle exceeds 1 MiB. Put large files in the project and use --asset."
        raise AnimationError(msg)
    bundle = json.loads(raw)
    if not isinstance(bundle, dict):
        msg = "Expected a JSON object with entry and files."
        raise AnimationError(msg)
    entry = bundle.get("entry")
    files = bundle.get("files")
    from_job = bundle.get("from_job")
    if not isinstance(entry, str) or not isinstance(files, dict):
        msg = "Expected a string entry and a files object with text contents."
        raise AnimationError(msg)
    staged = {}
    for path, content in files.items():
        if isinstance(content, str):
            staged[positive_path(path)] = content.encode()
    scene_path = positive_path(entry)
    if len(staged) != len(files):
        msg = "Every inline path must be unique and contain text."
        raise AnimationError(msg)
    if isinstance(from_job, str):
        staged = saved_files(root, from_job) | staged
    if scene_path not in staged or sum(map(len, staged.values())) > INPUT_LIMIT:
        msg = "Include the scene entry and keep total source and assets under 32 MiB."
        raise AnimationError(msg)
    return scene_path, staged


def load_scene(root: Path, args: argparse.Namespace) -> Scene:
    assets = [positive_path(value) for value in args.asset]
    if args.file == "-":
        entry, files = inline_files(root)
    else:
        entry, files = positive_path(args.file), {}
        assets = [entry, *assets]
    if entry.suffix != ".py" or not args.scene.isidentifier():
        msg = "Pass a .py scene file and the Python class name of the scene."
        raise AnimationError(msg)
    if len(files) + len(assets) > MAX_FILES:
        msg = f"A scene can include at most {MAX_FILES} source and asset files."
        raise AnimationError(msg)
    remaining = INPUT_LIMIT - sum(map(len, files.values()))
    for path in assets:
        if path in files:
            msg = f"The source or asset was supplied twice: {path}"
            raise AnimationError(msg)
        target = (root / path).resolve(strict=True)
        if not target.is_relative_to(root):
            msg = f"Asset resolves outside the project: {path}"
            raise AnimationError(msg)
        data = read_regular(target, remaining)
        files[path] = data
        remaining -= len(data)
    return Scene(entry, args.scene, files)


def snapshot(scene: Scene) -> str:
    state = state_folder(Path.cwd(), STATE)
    snapshots = state_folder(state, "snapshots")
    with tempfile.NamedTemporaryFile(dir=snapshots) as pending:
        with ZipFile(pending, "w") as bundle:
            bundle.comment = json.dumps([scene.entry.as_posix(), scene.name]).encode()
            for path, data in sorted(scene.files.items()):
                bundle.writestr(ZipInfo(path.as_posix()), data)
        pending.seek(0)
        digest = hashlib.sha256(pending.read()).hexdigest()
        with contextlib.suppress(FileExistsError):
            os.link(pending.name, snapshots / digest)
    return digest


def frozen_scene(state: Path, digest: str) -> Scene:
    if not re.fullmatch(r"[a-f0-9]{64}", digest):
        msg = "Invalid animation snapshot ID."
        raise AnimationError(msg)
    archive = read_regular(
        state_folder(state, "snapshots") / digest, INPUT_LIMIT + 2 * INLINE_LIMIT
    )
    if hashlib.sha256(archive).hexdigest() != digest:
        msg = "The saved animation snapshot has changed."
        raise AnimationError(msg)
    with ZipFile(io.BytesIO(archive)) as bundle:
        entry, name = json.loads(bundle.comment)
        return Scene(positive_path(entry), name, archive_files(bundle))


def podman_status() -> str | None:
    if not shutil.which(PODMAN[0]):
        return "Install Podman and configure rootless mode with cgroups v2."
    try:
        result = subprocess.run(  # noqa: S603
            [*PODMAN, "info", "--format", "{{.Host.Security.Rootless}} {{.Host.CgroupsVersion}}"],
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            timeout=10,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        return f"Podman could not start: podman info gave no answer in {error.timeout:g} seconds."
    if result.returncode:
        return f"Podman could not start: {result.stderr.strip()}"
    if result.stdout.strip() != "true v2":
        return "xpd animate needs rootless Podman with cgroups v2 on Linux."
    return None


def image_present() -> bool:
    return (
        subprocess.run(  # noqa: S603
            [*PODMAN, "image", "exists", IMAGE],
            stdin=subprocess.DEVNULL,
            capture_output=True,
            timeout=10,
            check=False,
        ).returncode
        == 0
    )


def doctor() -> dict[str, str | bool | None]:
    reason = podman_status()
    if reason is None and not image_present():
        reason = "Run xpd animate setup once to download the pinned Manim image."
    return {"ready": reason is None, "version": VERSION, "image": IMAGE, "reason": reason}


def host_memory_gib() -> int:
    try:
        for line in Path("/proc/meminfo").read_text(encoding="ascii").splitlines():
            if line.startswith("MemTotal:"):
                kib = int(line.split()[1])
                return max(2, kib // (1024 * 1024))
    except (OSError, ValueError, IndexError):
        pass
    return 8


def worker_limits(profile: str | None) -> tuple[int, str, int]:
    cpus = min(max(2, (os.cpu_count() or 4) // 4), 8)
    half_ram = max(2, host_memory_gib() // 2)
    if profile == "final":
        memory_gib = min(7, half_ram)
        timeout = 300
    elif profile in {"draft", "preview", "still"}:
        memory_gib = min(4, half_ram)
        timeout = 300
    else:
        memory_gib = min(2, half_ram)
        timeout = 120
    return cpus, f"{memory_gib}g", timeout


def worker(name: str, *mounts: tuple[Path, str, str], profile: str | None = None) -> list[str]:
    cpus, memory, timeout = worker_limits(profile)
    command = [
        *PODMAN,
        "run",
        "--rm",
        "--pull=never",
        "--name",
        name,
        "--network=none",
        "--http-proxy=false",
        "--log-driver=none",
        "--userns=keep-id:uid=1000,gid=1000",
        "--user=1000:1000",
        "--read-only",
        "--read-only-tmpfs=false",
        "--tmpfs=/tmp:rw,noexec,nosuid,nodev,size=1024m,mode=1777,notmpcopyup",
        "--cap-drop=all",
        "--security-opt=no-new-privileges",
        "--pids-limit=256",
        f"--memory={memory}",
        f"--cpus={cpus}",
        f"--timeout={timeout}",
        "--ulimit=nofile=256:256",
        f"--ulimit=fsize={VIDEO_LIMIT}:{VIDEO_LIMIT}",
        "--env=HOME=/tmp",
        "--env=XDG_CACHE_HOME=/tmp/cache",
        "--env=PYTHONHASHSEED=0",
        "--env=OPENBLAS_NUM_THREADS=1",
        "--env=OMP_NUM_THREADS=1",
        "--workdir=/",
    ]
    for source, target, mode in mounts:
        command.extend(["--volume", f"{source}:{target}:{mode}"])
    return [*command, IMAGE]


def setup() -> None:
    if reason := podman_status():
        raise AnimationError(reason)
    if not image_present():
        subprocess.run(  # noqa: S603
            [*PODMAN, "pull", IMAGE],
            stdin=subprocess.DEVNULL,
            timeout=900,
            check=True,
        )
    subprocess.run(  # noqa: S603
        [*worker(f"xpd-animate-{uuid4().hex}"), "manim", "--version"],
        stdin=subprocess.DEVNULL,
        timeout=15,
        check=True,
    )


def stop_worker(process: subprocess.Popen[bytes], name: str) -> None:
    if process.poll() is not None:
        return
    try:
        subprocess.run(  # noqa: S603
            [*PODMAN, "rm", "--force", "--time=0", "--ignore", name],
            stdin=subprocess.DEVNULL,
            capture_output=True,
            check=False,
            timeout=1,
        )
    finally:
        process.kill()
        process.wait()


def drain_export(process: subprocess.Popen[bytes], export: Path, log: Path) -> bytes:
    stdout, stderr = process.stdout, process.stderr
    if stdout is None or stderr is None:
        msg = "Could not capture Manim output."
        raise AnimationError(msg)
    tail = b""
    received = 0
    try:
        with stdout, stderr, selectors.DefaultSelector() as selector, export.open("wb") as archive:
            selector.register(stdout, selectors.EVENT_READ)
            selector.register(stderr, selectors.EVENT_READ)
            while selector.get_map():
                for key, _ in selector.select():
                    chunk = os.read(key.fd, 65536)
                    if not chunk:
                        selector.unregister(key.fileobj)
                    elif key.fileobj is stderr:
                        tail = (tail + chunk)[-LOG_LIMIT:]
                    else:
                        received += len(chunk)
                        if received > EXPORT_LIMIT:
                            msg = "Manim exceeded the render export limit."
                            raise AnimationError(msg)
                        archive.write(chunk)
    finally:
        log.write_bytes(tail)
    return tail


def capture(command: list[str], name: str, log: Path, export: Path) -> None:
    process = subprocess.Popen(  # noqa: S603
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        tail = drain_export(process, export, log)
        if process.wait(timeout=10):
            lines = [line.strip() for line in tail.decode(errors="replace").splitlines()]
            excerpt = "\n".join(line[:300] for line in lines[-8:] if line)[-1300:]
            msg = f"Manim render failed.\n{excerpt}"
            raise AnimationError(msg)
    finally:
        stop_worker(process, name)


def receive_export(export: Path, output: Path, segments: Path, media_format: str) -> None:
    artifact = f"artifact.{media_format}"
    media_limit = PNG_LIMIT if media_format == "png" else VIDEO_LIMIT
    remaining = SEGMENT_LIMIT
    try:
        with tarfile.open(export, "r:") as archive:
            for index, member in enumerate(archive):
                folder, prefix, name = member.name.rpartition("/")
                if not prefix and name == artifact:
                    destination, limit = output / name, media_limit
                elif folder == "segments" and SEGMENT.fullmatch(name):
                    destination, limit = segments / name, remaining
                    remaining -= member.size
                else:
                    msg = f"Unexpected file in Manim export: {member.name!r}"
                    raise AnimationError(msg)
                if (
                    index > MAX_FILES
                    or member.type != tarfile.REGTYPE
                    or not 0 < member.size <= limit
                ):
                    msg = "Manim export has an invalid file or exceeds its limits."
                    raise AnimationError(msg)
                stream = archive.extractfile(member)
                if stream is None:
                    msg = f"Could not read Manim export file: {member.name!r}"
                    raise AnimationError(msg)
                with stream, destination.open("xb") as file:
                    shutil.copyfileobj(stream, file)
    except tarfile.TarError as error:
        msg = f"Could not read Manim export: {error}"
        raise AnimationError(msg) from error


def state_folder(parent: Path, name: str) -> Path:
    folder = parent / name
    folder.mkdir(exist_ok=True)
    if folder.is_symlink() or not folder.is_dir():
        msg = f"Expected an ordinary xpd animation directory: {folder}"
        raise AnimationError(msg)
    return folder


def cached_report(jobs: Path, lookup: Path, fingerprint: str) -> RenderReport | None:
    try:
        job_id = read_regular(lookup, 32).decode()
    except (OSError, ValueError):
        return None
    return verified_report(jobs, job_id, fingerprint)


def verified_report(jobs: Path, job_id: str, fingerprint: str) -> RenderReport | None:
    try:
        folder = jobs / job_id
        if not JOB_ID.fullmatch(job_id) or folder.is_symlink():
            return None
        report = json.loads(read_regular(folder / "manifest.json", 256 * 1024))
        hashes = report["artifact_hashes"]
        expected = (
            {"still.png", "source.zip"}
            if report["profile"] == "still"
            else {"video.mp4", "contact-sheet.png", "source.zip"}
        )
        if (
            report["fingerprint"] != fingerprint
            or report["id"] != job_id
            or hashes.keys() != expected
        ):
            return None
        for filename, digest in hashes.items():
            data = read_regular(folder / positive_path(filename), VIDEO_LIMIT)
            if hashlib.sha256(data).hexdigest() != digest:
                return None
    except (OSError, AnimationError, KeyError, TypeError, ValueError):
        return None
    else:
        return report


def copy_segments(source: Path, destination: Path) -> None:
    if not source.is_dir() or source.is_symlink():
        return
    remaining = SEGMENT_LIMIT
    for path in sorted(source.iterdir())[:MAX_FILES]:
        if not SEGMENT.fullmatch(path.name):
            continue
        try:
            data = read_regular(path, remaining)
        except (OSError, AnimationError):
            continue
        target = destination / path.name
        if not target.exists():
            with tempfile.NamedTemporaryFile(dir=destination) as pending:
                pending.write(data)
                pending.flush()
                with contextlib.suppress(FileExistsError):
                    os.link(pending.name, target)
        remaining -= len(data)


def stage_source(scene: Scene, folder: Path) -> None:
    folder.mkdir()
    for name, data in sorted(scene.files.items()):
        path = folder / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def stage_config(scene: Scene, destination: Path) -> None:
    config = ConfigParser(interpolation=None)
    path = scene.entry.with_name("manim.cfg")
    if path not in scene.files:
        path = Path("manim.cfg")
    if path in scene.files:
        try:
            config.read_string(scene.files[path].decode("utf-8"), source=path.as_posix())
        except (ConfigError, UnicodeError) as error:
            msg = f"Invalid bundled Manim config {path}: {error}"
            raise AnimationError(msg) from error
    config.remove_section("CLI_CTX")
    config.read_string(CONFIG)
    with destination.open("w", encoding="utf-8") as stream:
        config.write(stream)


def unpack_result(
    task: Path, output: Path, entry: str, profile: str
) -> tuple[MediaStats, dict[str, bytes]]:
    quality, format_name = PROFILES[profile]
    inspection = task / "inspection"
    inspection.mkdir()
    media = "artifact.png" if format_name == "png" else "artifact.mp4"
    read_regular(output / media, PNG_LIMIT if format_name == "png" else VIDEO_LIMIT)
    name = f"xpd-inspect-{uuid4().hex}"
    command = [
        *worker(
            name,
            (output, "/media", "ro"),
            (inspection, "/inspection", "rw"),
        ),
        "python",
        "-I",
        "-c",
        sys.argv[1],
        media,
        quality,
    ]
    process = subprocess.Popen(  # noqa: S603
        command,
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        stdout, stderr = process.communicate(timeout=120)
    finally:
        stop_worker(process, name)
    if process.returncode:
        detail = stderr.decode(errors="replace").strip()[-800:]
        msg = f"Manim could not produce a valid {profile} artifact for {entry}: {detail}"
        raise AnimationError(msg)
    stats: MediaStats = json.loads(stdout)
    if format_name == "png":
        return stats, {"still.png": read_regular(output / media, PNG_LIMIT)}
    return stats, {
        "video.mp4": read_regular(output / media, VIDEO_LIMIT),
        "contact-sheet.png": read_regular(inspection / "contact-sheet.png", PNG_LIMIT),
    }


def run_scene(
    task: Path, scene: Scene, profile: str, log: Path, partial_cache: Path | None
) -> tuple[MediaStats, dict[str, bytes]]:
    source = task / "source"
    stage_source(scene, source)
    configuration = task / "manim.cfg"
    stage_config(scene, configuration)
    shim = task / "animation_worker.py"
    shim.write_text(sys.argv[2], encoding="utf-8")
    output = task / "output"
    output.mkdir()
    seed = task / "seed"
    seed.mkdir()
    if partial_cache is not None:
        copy_segments(partial_cache, seed)
    segments = task / "segments"
    segments.mkdir()
    quality, format_name = PROFILES[profile]
    name = f"xpd-animate-{uuid4().hex}"
    command = [
        *worker(
            name,
            (source, "/source", "ro"),
            (configuration, "/xpd-manim.cfg", "ro"),
            (seed, "/seed", "ro"),
            (shim, "/xpd-animation-worker.py", "ro"),
            profile=profile,
        ),
        "python",
        "-I",
        "/xpd-animation-worker.py",
        scene.entry.as_posix(),
        scene.name,
        quality,
        format_name,
        str(PNG_LIMIT if format_name == "png" else VIDEO_LIMIT),
        str(SEGMENT_LIMIT),
        str(MAX_FILES if partial_cache is not None else 0),
    ]
    export = task / "export.tar"
    capture(command, name, log, export)
    receive_export(export, output, segments, format_name)
    result = unpack_result(task, output, str(scene.entry), profile)
    if partial_cache is not None:
        copy_segments(segments, partial_cache)
    return result


def publish(
    pending: Path, plan: Render, stats: MediaStats, artifacts: dict[str, bytes]
) -> RenderReport:
    base = plan.path
    scene = plan.scene
    for filename, data in artifacts.items():
        (pending / filename).write_bytes(data)
    stage_source(scene, pending / "source")
    with ZipFile(pending / "source.zip", "w", compression=ZIP_DEFLATED) as bundle:
        for path, data in sorted(scene.files.items()):
            bundle.writestr(path.as_posix(), data)
    hashes = {name: hashlib.sha256(data).hexdigest() for name, data in artifacts.items()}
    hashes["source.zip"] = hashlib.sha256((pending / "source.zip").read_bytes()).hexdigest()
    report: RenderReport = {
        "id": plan.job_id,
        "scene": scene.name,
        "entry": scene.entry.as_posix(),
        "files": sorted(path.as_posix() for path in scene.files),
        "profile": plan.profile,
        "renderer": f"Manim Community {VERSION} (Cairo)",
        **stats,
        "video": str(base / "video.mp4") if "video.mp4" in artifacts else None,
        "still": str(base / "still.png") if "still.png" in artifacts else None,
        "contact_sheet": str(base / "contact-sheet.png")
        if "contact-sheet.png" in artifacts
        else None,
        "source": str(base / "source.zip"),
        "source_dir": str(base / "source"),
        "manifest": str(base / "manifest.json"),
        "log": str(base / "render.log"),
        "elapsed_seconds": plan.elapsed(),
        "cached": False,
        "fingerprint": plan.fingerprint,
        "artifact_hashes": hashes,
    }
    (pending / "manifest.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


def render_scene(plan: Render, *, use_cache: bool, staging: Path | None = None) -> RenderReport:
    root = Path.cwd()
    scene = plan.scene
    fingerprint = plan.fingerprint
    state = root / STATE
    jobs = state / "jobs"
    cache = state / "cache"
    if (
        use_cache
        and not any(path.is_symlink() for path in (state, jobs, cache))
        and (report := cached_report(jobs, cache / fingerprint, fingerprint))
    ):
        return {**report, "cached": True, "elapsed_seconds": plan.elapsed()}

    status = doctor()
    if isinstance(status["reason"], str):
        raise AnimationError(status["reason"])
    state = state_folder(root, STATE)
    jobs = state_folder(state, "jobs")
    lookup = state_folder(state, "cache") / fingerprint
    partial_cache = None
    if use_cache or staging is not None:
        partial_cache = state_folder(
            state_folder(state, "segments"), scene.segment_key(plan.profile)
        )
    final = root / plan.path
    with (
        tempfile.TemporaryDirectory(prefix="xpd-manim-", dir=staging) as scratch,
        tempfile.TemporaryDirectory(prefix=".pending-", dir=staging or jobs) as pending_name,
    ):
        pending = Path(pending_name)
        log = pending / "render.log"
        try:
            stats, artifacts = run_scene(Path(scratch), scene, plan.profile, log, partial_cache)
            report = publish(pending, plan, stats, artifacts)
        except (OSError, AnimationError, subprocess.SubprocessError) as error:
            if log.exists():
                if staging is not None:
                    failure_log = state_folder(state, "logs") / f"{plan.job_id}.log"
                    log.replace(failure_log)
                else:
                    pending.rename(final)
                    failure_log = final / log.name
                msg = f"{error}\nRender log: {failure_log.relative_to(root)}"
                raise AnimationError(msg) from error
            raise
        pending.rename(final)
    if use_cache:
        index_update = lookup.with_name(plan.job_id)
        index_update.write_text(plan.job_id, encoding="ascii")
        index_update.replace(lookup)
    return report


def render_snapshot(digest: str, job_id: str, profile: str) -> RenderReport:
    if not JOB_ID.fullmatch(job_id):
        msg = "Invalid workflow render job ID."
        raise AnimationError(msg)
    state = state_folder(Path.cwd(), STATE)
    scene = frozen_scene(state, digest)
    plan = Render(scene, profile, job_id, scene.fingerprint(profile), time.monotonic())
    locks = state_folder(state, "locks")
    flags = os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW | os.O_NONBLOCK
    with os.fdopen(os.open(locks / job_id, flags, 0o600), "r+b") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        staging = state_folder(state, "staging") / job_id
        if staging.exists():
            shutil.rmtree(staging)
        jobs = state_folder(state, "jobs")
        if os.path.lexists(jobs / job_id):
            if report := verified_report(jobs, job_id, plan.fingerprint):
                return report
            msg = f"Published workflow render failed verification: {job_id}"
            raise AnimationError(msg)
        staging.mkdir(mode=0o700)
        try:
            return render_scene(plan, use_cache=False, staging=staging)
        finally:
            shutil.rmtree(staging)


def cancelled(_signal: int, _frame: object) -> None:
    raise KeyboardInterrupt


def render(args: argparse.Namespace) -> None:
    started = time.monotonic()
    scene = load_scene(Path.cwd(), args)
    plan = Render(scene, args.profile, uuid4().hex, scene.fingerprint(args.profile), started)
    report = render_scene(plan, use_cache=not args.no_cache)
    if args.json:
        print(json.dumps(report))
    else:
        print(report["video"] or report["still"])
        print(f"Editable scene: {report['source_dir']}/{report['entry']}")
        print(f"Render settings: {report['manifest']}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(prog="xpd animate")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("setup")
    commands.add_parser("doctor").add_argument("--json", action="store_true")
    render_args = commands.add_parser("render")
    render_args.add_argument("--profile", choices=PROFILES, required=True)
    render_args.add_argument("--json", action="store_true")
    render_args.add_argument("--no-cache", action="store_true")
    snapshot_args = commands.add_parser("snapshot")
    for source_args in (render_args, snapshot_args):
        source_args.add_argument("--asset", action="append", default=[])
        source_args.add_argument("file")
        source_args.add_argument("scene")
    frozen = commands.add_parser("render-snapshot")
    frozen.add_argument("snapshot")
    frozen.add_argument("job_id")
    frozen.add_argument("profile", choices=("preview", "final"))
    return parser.parse_args(sys.argv[4:])


def main() -> int:
    args = parse_args()
    signal.signal(signal.SIGTERM, cancelled)
    try:
        match args.command:
            case "setup":
                setup()
            case "doctor":
                status = doctor()
                if args.json:
                    print(json.dumps(status))
                else:
                    print(status["reason"] or f"Ready: Manim Community {VERSION}, rootless Podman")
            case "snapshot":
                scene = load_scene(Path.cwd(), args)
                print(json.dumps({"snapshot": snapshot(scene)}))
            case "render-snapshot":
                report = render_snapshot(args.snapshot, args.job_id, args.profile)
                print(json.dumps(report))
            case "render":
                render(args)
    except (AnimationError, BadZipFile, OSError, ValueError, subprocess.SubprocessError) as error:
        if getattr(args, "json", False):
            print(json.dumps({"error": str(error)}))
        else:
            print(f"xpd animate: {error}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())

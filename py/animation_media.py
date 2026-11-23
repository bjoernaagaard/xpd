"""Decode Manim output inside the worker and sample a short contact sheet."""

import json
import sys
from collections.abc import Iterable, Iterator
from pathlib import Path
from typing import TypedDict

from av import open as open_media
from av.video.frame import VideoFrame
from manim.constants import QUALITIES
from PIL import Image, ImageDraw

SAMPLES = (0.1, 0.5, 0.9)
THUMBNAIL_WIDTH = 320


class MediaStats(TypedDict):
    width: int
    height: int
    fps: int | None
    duration_seconds: float | None


def timed_frames(frames: Iterable[VideoFrame], fps: int) -> Iterator[tuple[float, VideoFrame]]:
    start = None
    for index, frame in enumerate(frames):
        timestamp = frame.time
        if timestamp is None:
            msg = f"Frame {index} has no timestamp."
            raise ValueError(msg)
        if start is None:
            start = timestamp
        elapsed = timestamp - start
        if abs(elapsed - index / fps) > 1 / fps:
            msg = f"Frame {index} arrived at {elapsed:.3f}s, outside the {fps} fps timeline."
            raise ValueError(msg)
        yield elapsed, frame


def contact_sheet(path: Path, width: int, height: int, fps: int) -> MediaStats:
    thumbnail_height = round(height * THUMBNAIL_WIDTH / width)
    sheet = Image.new("RGB", (THUMBNAIL_WIDTH * len(SAMPLES), thumbnail_height + 24), "#10151e")
    labels = ImageDraw.Draw(sheet)

    with open_media(str(path)) as container:
        if not container.streams.video:
            msg = "Expected a video stream."
            raise ValueError(msg)
        stream = container.streams.video[0]
        if (stream.width, stream.height) != (width, height):
            msg = f"Expected {width}x{height}, got {stream.width}x{stream.height}."
            raise ValueError(msg)
        expected_frames = stream.frames
        if expected_frames <= 0:
            msg = "The video has no declared frames."
            raise ValueError(msg)
        targets = [round((expected_frames - 1) * fraction) for fraction in SAMPLES]
        decoded = 0
        for index, (elapsed, frame) in enumerate(timed_frames(container.decode(video=0), fps)):
            decoded += 1
            for column, target in enumerate(targets):
                if index == target:
                    thumbnail = frame.to_image().resize(
                        (THUMBNAIL_WIDTH, thumbnail_height), Image.Resampling.LANCZOS
                    )
                    left = column * THUMBNAIL_WIDTH
                    sheet.paste(thumbnail, (left, 0))
                    labels.text((left + 10, thumbnail_height + 6), f"{elapsed:.2f}s", fill="white")
        if expected_frames != decoded:
            msg = f"Expected {expected_frames} frames, decoded {decoded}."
            raise ValueError(msg)
        duration = decoded / fps
        if stream.duration is not None and stream.time_base is not None:
            reported = float(stream.duration * stream.time_base)
            if abs(reported - duration) > 1 / fps:
                msg = f"The encoded duration {reported:.3f}s differs from {duration:.3f}s."
                raise ValueError(msg)
            duration = reported

    sheet.save("/inspection/contact-sheet.png")
    return {"width": width, "height": height, "fps": fps, "duration_seconds": duration}


def main() -> None:
    name, flag = sys.argv[1:]
    quality = next(spec for spec in QUALITIES.values() if spec["flag"] == flag.removeprefix("-q"))
    width, height, fps = quality["pixel_width"], quality["pixel_height"], quality["frame_rate"]
    path = Path("/media") / name
    if path.suffix == ".png":
        with Image.open(path) as image:
            if image.size != (width, height):
                msg = f"Expected a {width}x{height} still image."
                raise ValueError(msg)
            image.load()
        stats: MediaStats = {
            "width": width,
            "height": height,
            "fps": None,
            "duration_seconds": None,
        }
    else:
        stats = contact_sheet(path, width, height, fps)
    print(json.dumps(stats))


if __name__ == "__main__":
    main()

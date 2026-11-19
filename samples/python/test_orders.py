"""Ways a test hides a failure that xpd reports, and the same tests without them.

A line that xpd reports ends with a comment that names the rule.
"""

import sys
import tempfile
import threading
import time
from pathlib import Path

import pytest
from flaky import flaky  # xpd: test-rerun
from hypothesis import HealthCheck, given, settings
from hypothesis import strategies as st


def start(done: threading.Event) -> threading.Thread:
    worker = threading.Thread(target=done.set)
    worker.start()
    return worker


@pytest.mark.skip(reason="fails on CI")  # xpd: test-skip
def test_total_skipped() -> None:
    assert sum([1.0, 2.0]) == 3.0


@pytest.mark.flaky(reruns=3)  # xpd: test-rerun
def test_worker_with_reruns() -> None:
    done = threading.Event()
    start(done)
    time.sleep(0.5)  # xpd: test-sleep
    assert done.is_set()


@flaky
def test_worker_with_flaky() -> None:
    done = threading.Event()
    start(done).join()
    assert done.is_set()


@pytest.mark.skipif(sys.platform == "win32", reason="needs fork")
def test_worker_waits_for_the_result() -> None:
    done = threading.Event()
    start(done)
    assert done.wait(timeout=5)


@pytest.mark.xfail(reason="rounding", strict=False)  # xpd: test-skip
def test_total_may_fail() -> None:
    assert sum([0.1, 0.2]) == 0.3


@pytest.mark.xfail(reason="rounding")
def test_total_fails_until_fixed() -> None:
    assert sum([0.1, 0.2]) == 0.3


@settings(suppress_health_check=[HealthCheck.function_scoped_fixture])  # xpd: test-suppressed-health-check
@given(st.text())
def test_note_shares_a_folder(tmp_path: Path, text: str) -> None:
    path = tmp_path / "note.txt"
    path.write_text(text, encoding="utf-8")
    assert path.read_text(encoding="utf-8") == text


@settings(deadline=None, suppress_health_check=[HealthCheck.too_slow])
@given(st.text())
def test_note_gets_its_own_folder(text: str) -> None:
    with tempfile.TemporaryDirectory() as folder:
        path = Path(folder) / "note.txt"
        path.write_text(text, encoding="utf-8")
        assert path.read_text(encoding="utf-8") == text

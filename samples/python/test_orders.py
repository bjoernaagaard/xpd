"""Ways a test hides a failure that xpd reports, and the same tests without them.

A line that xpd reports ends with a comment that names the rule.
"""

import sys
import threading
import time

import pytest
from flaky import flaky  # xpd: test-rerun


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

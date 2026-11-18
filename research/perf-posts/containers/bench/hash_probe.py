"""Count element hash calls on first use and reuse of the same tuple."""

import json
import platform
import sys
import tracemalloc


class Key:
    calls = 0

    def __init__(self, value):
        self.value = value

    def __hash__(self):
        Key.calls += 1
        return hash(self.value)


rows = []
for size in (10, 1000, 100000):
    for kind in (int, str):
        items = [Key(kind(i)) for i in range(size)]
        tracemalloc.start()
        key = tuple(items)
        peak = tracemalloc.get_traced_memory()[1]
        tracemalloc.stop()
        Key.calls = 0
        first = hash(key)
        first_calls = Key.calls
        second = hash(key)
        assert first == second and first_calls == size
        rows.append(
            {
                "size": size,
                "kind": kind.__name__,
                "first_calls": first_calls,
                "repeat_calls": Key.calls - first_calls,
                "bytes": sys.getsizeof(key),
                "peak_construction_bytes": peak,
            }
        )
print(json.dumps({"python": platform.python_version(), "rows": rows}, indent=1))

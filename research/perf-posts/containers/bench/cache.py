"""Load the installed Python fallback as well as the normal C cache."""

import functools
import runpy
import sys


def functions(size, kind):
    accelerator = sys.modules.pop("_functools")
    sys.modules["_functools"] = None
    try:
        fallback = runpy.run_path(functools.__file__)["lru_cache"]
    finally:
        sys.modules["_functools"] = accelerator

    def work(a, b, c, d, e):
        pass

    forms = {}
    first = tuple(map(kind, (1, 2, 3, 4, 5)))
    second = tuple(map(kind, (1, 2, 3, 4, 6)))
    for name, decorator in (("python", fallback), ("C", functools.lru_cache)):
        cached = decorator(maxsize=1)(work)

        def call(cached=cached):
            for _ in range(size):
                cached(*first)
                cached(*second)

        call()
        assert cached.cache_info().misses == 2 * size and cached.cache_info().hits == 0
        forms[name] = call
    return forms

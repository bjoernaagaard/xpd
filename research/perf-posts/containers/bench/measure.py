"""Interleave equivalent operations and retain every pyperf sample."""

import argparse
from collections import Counter, defaultdict, deque
import heapq
import json
import math
import operator
import os
import platform
import random
import statistics
import subprocess
import sys
import time
import timeit
import tracemalloc
from pathlib import Path

import pyperf
from cache import functions as cache_functions
from heapsort import heapsort_iter
from merge import inputs, merge, wrap


def cases(study, n, kind, input_bytes):
    rng = random.Random(1000)
    if study == "mdp":
        from mdp import Battle
        assert abs(Battle().evaluate(0.192) - 0.89873589887) < 1e-6
        return {"MDP": lambda: Battle().evaluate(0.192)}
    if study == "group":
        pairs = [(kind(i % 50), i) for i in range(n)]

        def using_setdefault():
            groups = {}
            for key, value in pairs:
                groups.setdefault(key, []).append(value)
            return groups

        def using_defaultdict():
            groups = defaultdict(list)
            for key, value in pairs:
                groups[key].append(value)
            return groups

        return {"setdefault": using_setdefault, "defaultdict": using_defaultdict}
    if study == "cache":
        return cache_functions(n, kind)
    if study == "merge":
        a, b = inputs(n, kind)
        return {
            "sort": lambda: sorted(a + b),
            "mine": lambda: list(merge(wrap(a), wrap(b))),
            "heap": lambda: list(heapq.merge(a, b)),
        }
    if study == "heapsort":
        data = list(zip(kind(rng.randint(0, n // 10)) for _ in range(n)))
        return {"sort": lambda: sorted(data), "heap": lambda: list(heapsort_iter(data))}
    if study == "membership":
        sequence = list(map(kind, range(n)))
        mapping = dict.fromkeys(sequence)
        input_bytes.update(list=sys.getsizeof(sequence), dict=sys.getsizeof(mapping))
        missing = kind(-1)
        return {"list": lambda: missing in sequence, "dict": lambda: missing in mapping}
    if study == "count":
        rng.seed(42)
        pool = [
            "".join(rng.choice("abcdefghijklmnopqrstuvwxyz") for _ in range(rng.randint(3, 9)))
            for _ in range(5000)
        ]
        if kind is int:
            words = [i % 50 for i in range(n)]
        else:
            words = [rng.choice(pool) for _ in range(n)]

        def tally():
            counts = {}
            for word in words:
                counts[word] = counts.get(word, 0) + 1
            return counts

        return {"get": tally, "Counter": lambda: Counter(words)}
    if study == "deque":
        data = list(map(kind, range(n)))

        def grow(container, append):
            elapsed = 0
            for item in data:
                start = time.perf_counter()
                append(item)
                elapsed += time.perf_counter() - start
            return container, elapsed / len(data)

        def front_list():
            seq = []
            return grow(seq, lambda i: seq.insert(0, i))

        def front_deque():
            seq = deque()
            return grow(seq, lambda i: seq.appendleft(i))

        return {"list": front_list, "deque": front_deque}
    raise ValueError(study)


def load():
    return subprocess.check_output(["cut", "-d ", "-f1-3", "/proc/loadavg"], text=True).strip()


def measure(study, size, kind):
    input_bytes = {}
    forms = cases(study, size, kind, input_bytes)
    results = []
    memory = {}
    for name, func in forms.items():
        result = func()
        results.append(list(result[0]) if study == "deque" else result)
        if study == "mdp":
            continue
        tracemalloc.start()
        value = func()
        peak = tracemalloc.get_traced_memory()[1]
        tracemalloc.stop()
        memory[name] = dict(
            peak_bytes=peak, shallow_bytes=sys.getsizeof(value[0] if study == "deque" else value)
        )
    assert all(value == results[0] for value in results)
    if study == "heapsort":
        assert all(map(operator.is_, *results))
    del results, result
    calibration_load = load()
    loops = {
        name: max(1, min(1000000, math.ceil(0.01 / timeit.Timer(func).timeit(1))))
        for name, func in forms.items()
    }
    attempts = []
    for attempt in range(3):
        samples = {name: [] for name in forms}
        loads = []
        for i in range(3):
            names = list(forms)
            for name in names[i % len(names) :] + names[: i % len(names)]:
                loads.append(load())
                if study == "deque":
                    sample = statistics.mean(forms[name]()[1] for _ in range(loops[name]))
                else:
                    sample = timeit.Timer(forms[name]).timeit(loops[name]) / loops[name]
                samples[name].append(sample)
        benches = [
            pyperf.Benchmark(
                [
                    pyperf.Run(
                        values,
                        metadata={"name": name, "unit": "second", "loops": loops[name]},
                        collect_metadata=False,
                    )
                ]
            )
            for name, values in samples.items()
        ]
        stats = {b.get_name(): {"mean_s": b.mean(), "sd_s": b.stdev()} for b in benches}
        ref, *others = samples
        ratios = {
            name: statistics.mean(x / y for x, y in zip(samples[ref], samples[name]))
            for name in others
        }
        noise = max(b.stdev() / b.mean() for b in benches)
        attempts.append(
            dict(loadavg=loads, samples=samples, stats=stats, ratios=ratios, max_cv=noise)
        )
        if noise <= 0.05:
            break
    best = min(range(len(attempts)), key=lambda i: attempts[i]["max_cv"])
    return dict(
        size=size,
        kind=kind.__name__,
        input_bytes=input_bytes,
        calibration_load=calibration_load,
        loops=loops,
        memory=memory,
        attempts=attempts,
        best=best,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "study", choices=["mdp", "merge", "heapsort", "count", "deque", "membership", "cache", "group"]
    )
    parser.add_argument("version")
    args = parser.parse_args()
    assert platform.python_version() == args.version and os.sched_getaffinity(0) == {8}
    grid = [(n, kind) for n in (10, 1000, 100000) for kind in (int, str)]
    primary = {
        "merge": (1000000, int),
        "heapsort": (100000, int),
        "count": (200000, str),
        "deque": (10000, int),
        "membership": (10000000, int),
        "cache": (1, int),
        "group": (100000, int),
    }
    primary["mdp"] = (1, int)
    if args.study == "mdp":
        grid = []
    grid = list(dict.fromkeys([primary[args.study], *grid]))
    record = dict(
        python=args.version, executable=sys.executable, cpu=list(os.sched_getaffinity(0)), rows=[]
    )
    for size, kind in grid:
        record["rows"].append(measure(args.study, size, kind))
        print(args.study, args.version, size, kind.__name__, flush=True)
    Path(f"runs/{args.study}-{args.version}.json").write_text(json.dumps(record, indent=1) + "\n")

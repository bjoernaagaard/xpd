"""Arrays built the slow way, copies nothing needs, and files the program writes and reads back.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import os
import tempfile

import numpy as np

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def from_python():
    n = 6
    counted = np.array(range(n))  # xpd: array-construction
    stepped = np.asarray(list(range(2, 12, 3)))  # xpd: array-construction
    zeros = np.array([0] * n)  # xpd: array-construction
    halves = np.array(n * [0.5])  # xpd: array-construction
    return counted, stepped, zeros, halves


@case
def written_twice():
    n = 5
    threes = np.ones(n) * 3  # xpd: array-construction
    negative = -1.5 * np.ones((2, 3))  # xpd: array-construction
    small = np.zeros(n).astype(np.float32)  # xpd: array-construction
    flags = np.ones((2, 2)).astype(bool)  # xpd: array-construction
    floats = np.arange(1, n + 1).astype(float)  # xpd: array-construction
    return threes, negative, small, flags, floats


@case
def meant_construction():
    n = 5
    sigma = np.float32(0.5)
    rows = np.array([[0] * 3] * 2)
    names = np.array(["a"] + ["b"] * n)
    scaled = np.ones(n) * sigma
    bigger = np.zeros(n) + 7
    typed = np.zeros(n).astype(np.dtype("f4"))
    labels = np.arange(n).astype(str)
    fractions = np.arange(0, 1, 0.25).astype(np.float32)
    explicit = np.arange(n, dtype=np.int32).astype(float)
    return rows, names, scaled, bigger, typed, labels, fractions, explicit


@case
def copies():
    m = np.arange(12.0).reshape(3, 4)
    flat = np.abs(m - 5).flatten()  # xpd: needless-copy
    joined = np.concatenate([m, m]).flatten()  # xpd: needless-copy
    narrow = m.copy().astype(np.float32)  # xpd: needless-copy
    owned = np.array(m).copy()  # xpd: needless-copy
    return flat, joined, narrow, owned


@case
def meant_copies():
    m = np.arange(12.0).reshape(3, 4)
    own = m.flatten()
    own[0] = -1
    view = np.asarray(m).flatten()
    view[1] = -1
    out = np.empty((3, 4))
    into = np.add(m, 1, out=out).flatten()
    maybe = m.copy().astype(np.float64, copy=False)
    kept = np.array(m, copy=True).copy()
    return own, view, into, maybe, kept, m


@case
def file_roundtrip():
    data = np.linspace(0, 1, 7)
    with tempfile.TemporaryDirectory() as tmp:
        text = os.path.join(tmp, "data.txt")
        np.savetxt(text, data)
        back = np.loadtxt(text)  # xpd: file-roundtrip
        again = np.genfromtxt(text)  # xpd: file-roundtrip
        raw = os.path.join(tmp, "data.bin")
        data.tofile(raw)
        loaded = np.fromfile(raw, dtype=np.float64)  # xpd: file-roundtrip
    return back, again, loaded


@case
def meant_files():
    data = np.linspace(0, 1, 7)
    with tempfile.TemporaryDirectory() as tmp:
        export = os.path.join(tmp, "export.csv")
        np.savetxt(export, data, delimiter=",")
        given = os.path.join(tmp, "given.txt")
        with open(given, "w") as f:
            f.write("1.0\n2.0\n")
        read = np.loadtxt(given)
        sensor = os.path.join(tmp, "sensor.bin")
        with open(sensor, "wb") as f:
            f.write(np.arange(4, dtype=np.int16).tobytes())
        samples = np.fromfile(sensor, dtype=np.int16)
    return read, samples

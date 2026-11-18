"""Rebuild the three timed expressions in Python Discuss post 93521/17."""

import random


class Inf:
    def __lt__(self, other):
        return False

    def __gt__(self, other):
        return True


INF = Inf()


def wrap(xs):
    yield from xs
    while True:
        yield INF


def merge(a, b):
    nexta = a.__next__
    nextb = b.__next__
    aval = nexta()
    bval = nextb()
    while True:
        if bval < aval:
            t = bval
            bval = nextb()
        else:
            t = aval
            aval = nexta()
        if t is INF:
            break
        yield t


def inputs(size, kind):
    rng = random.Random(31415926535)
    a, b = (sorted(kind(rng.randint(0, size)) for _ in range(size)) for _ in range(2))
    return a, b

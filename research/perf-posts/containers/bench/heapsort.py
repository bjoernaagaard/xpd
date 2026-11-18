"""Rebuild the stable heap iterator from Python Discuss post 93394/29."""

import heapq
import itertools
import operator


def heapsort_iter(arr):
    arr = list(zip(arr, itertools.count()))
    heapq.heapify(arr)
    return map(operator.itemgetter(0), map(heapq.heappop, itertools.repeat(arr, len(arr))))

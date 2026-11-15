"""Sparse habits that SciPy warns about or advises against.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import scipy
import scipy.sparse as sp
from scipy import sparse
from scipy.sparse import csc_array, csr_matrix, eye

CASES = {}


def case(fn):
    CASES[fn.__name__] = fn
    return fn


@case
def matrix_classes():
    d = np.array([[1.0, 0.0], [0.0, 2.0]])
    a = sp.csr_matrix(d)  # xpd: sparse-matrix-api
    b = csr_matrix(d)  # xpd: sparse-matrix-api
    c = scipy.sparse.lil_matrix((2, 2))  # xpd: sparse-matrix-api
    return a.toarray(), b.toarray(), c.toarray()


@case
def array_classes():
    d = np.array([[1.0, 0.0], [0.0, 2.0]])
    return sp.csr_array(d).toarray(), csc_array(d).toarray(), sparse.issparse(d)


@case
def matrix_builders():
    a = sp.eye(3)  # xpd: sparse-matrix-api
    b = eye(3)  # xpd: sparse-matrix-api
    c = sparse.diags([1.0, 2.0])  # xpd: sparse-matrix-api
    return a.toarray(), b.toarray(), c.toarray(), np.eye(3)


@case
def array_builders():
    return sp.eye_array(3).toarray(), sp.diags_array([1.0, 2.0]).toarray()


@case
def matrix_kept_for_old_scipy(old_scipy=False):
    d = np.eye(2)
    if old_scipy:
        return sp.csr_matrix(d).toarray()
    return sp.csr_array(d).toarray()


@case
def matrix_on_request(kind="matrix"):
    d = np.eye(2)
    if kind == "matrix":
        return sp.csr_matrix(d).toarray()
    return d


def empty_csr_writes(rows, cols, vals):
    a = sp.csr_matrix((4, 4))
    for i, j, v in zip(rows, cols, vals):
        a[i, j] = v  # xpd: empty-csr-write
    b = sparse.csc_array((4, 4), dtype=np.float32)
    b[0, 1] += 2.0  # xpd: empty-csr-write
    c = sp.csr_array(np.zeros((4, 4)).shape)
    c[1, :] = np.ones(4)  # xpd: empty-csr-write
    return a, b, c


@case
def writes_that_do_not_warn():
    rows, cols, vals = [0, 1], [1, 2], [3.0, 4.0]
    a = sp.lil_array((4, 4))
    for i, j, v in zip(rows, cols, vals):
        a[i, j] = v
    b = sp.csr_array(np.eye(4))
    b[0, 0] = 5.0
    c = sp.csr_array((np.array(vals), (np.array(rows), np.array(cols))), shape=(4, 4))
    c[0, 1] = 1.0
    d = sp.csr_array((4, 4))
    d = d.tolil()
    d[0, 1] = 1.0
    return a.tocsr().toarray(), b.toarray(), c.toarray(), d.toarray()

"""Sparse habits that SciPy warns about or advises against.

A line that xpd reports ends with a comment that names the rule. Each case returns what the
code computes, so a copy fixed with `xpd check --fix` can be compared with this file.
"""

import numpy as np
import scipy
import scipy.sparse as sp
from scipy import sparse
from scipy.sparse import csc_array, csr_matrix, eye
from scipy.sparse import linalg as spla
from scipy.sparse.linalg import inv, spsolve

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
def both_kinds_in_one_list():
    d = np.eye(2)
    return [sp.csr_matrix(d).toarray(), sp.csr_array(d).toarray()]


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
    a = sp.csr_matrix((4, 4))  # xpd: empty-csr-write
    for i, j, v in zip(rows, cols, vals):
        a[i, j] = v
    b = sparse.csc_array((4, 4), dtype=np.float32)  # xpd: empty-csr-write
    b[0, 1] += 2.0
    c = sp.csr_array(np.zeros((4, 4)).shape)  # xpd: empty-csr-write
    if len(rows):
        c[1, :] = np.ones(4)
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


def poisson(n):
    t = sp.diags_array([-1.0, 2.0, -1.0], offsets=[-1, 0, 1], shape=(n, n))
    return (sp.kron(sp.eye_array(n), t) + sp.kron(t, sp.eye_array(n))).tocsc()


@case
def sparse_inverse_then_product():
    a = poisson(4)
    b = np.arange(16.0)
    x = spla.inv(a) @ b  # xpd: sparse-inv-then-multiply
    y = inv(a).dot(b)  # xpd: sparse-inv-then-multiply
    z = spla.spsolve(a, b)
    w = spla.inv(a)
    return x, y, z, w.toarray()


@case
def spsolve_per_pass():
    a = poisson(4)
    sides = np.arange(48.0).reshape(3, 16)
    xs = [spla.spsolve(a, b) for b in sides]  # xpd: solve-in-loop
    ys = []
    for b in sides:
        ys.append(spsolve(a, b))  # xpd: solve-in-loop
    return np.array(xs), np.array(ys)


@case
def spsolve_with_a_new_matrix():
    a = poisson(4)
    eye16 = sp.eye_array(16, format="csc")
    sides = np.arange(48.0).reshape(3, 16)
    xs = [spla.spsolve(a + s * eye16, b) for s, b in zip((1.0, 2.0, 3.0), sides)]
    ys = []
    for m in (a, 2 * a):
        ys.append(spla.spsolve(m, sides[0]))
    lu = spla.splu(a)
    zs = [lu.solve(b) for b in sides]
    for k in range(2):
        a = a + eye16
        zs.append(spla.spsolve(a, sides[k]))
    return np.array(xs), np.array(ys), np.array(zs)

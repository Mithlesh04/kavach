# The product of two 1024 x 1024 float matrices, 4 times, with NumPy's
# `@` (BLAS: OpenBLAS when it is the BLAS loaded). The matrices are the
# ones the Kavach and C++ programs make. BLAS may add each item's
# products in any order, so the last bits may differ from theirs.
import numpy as np

n = 1024
i = np.arange(n * n)
a = ((i % 97) * 0.01).reshape(n, n)
b = ((i % 89) * 0.02 - 0.5).reshape(n, n)
total = 0.0
for r in range(4):
    c = a @ b
    total += c.flat[r * 1000]
print(repr(total))

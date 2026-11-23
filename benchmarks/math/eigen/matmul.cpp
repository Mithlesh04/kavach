// The product of two 1024 x 1024 float matrices, 4 times, with Eigen's
// `a * b` (its own blocked, vectorized product, on all cores through
// OpenMP). Row-major, as Kavach's tensors are. Eigen may add each
// item's products in another order, so the last bits may differ.
// Build: g++ -O2 -march=native -fopenmp -I/usr/include/eigen3
#include <Eigen/Dense>
#include <cstdio>

using Mat = Eigen::Matrix<double, Eigen::Dynamic, Eigen::Dynamic, Eigen::RowMajor>;

int main() {
    const int n = 1024;
    Mat a(n, n), b(n, n), c(n, n);
    for (int i = 0; i < n * n; i++) {
        a.data()[i] = (double)(i % 97) * 0.01;
        b.data()[i] = (double)(i % 89) * 0.02 - 0.5;
    }
    double total = 0.0;
    for (int round = 0; round < 4; round++) {
        c.noalias() = a * b;
        total += c.data()[round * 1000];
    }
    std::printf("%.17g\n", total);
}

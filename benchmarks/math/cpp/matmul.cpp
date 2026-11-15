// The product of two 1024 x 1024 float matrices, 4 times, on one core:
// each item adds its products in order, k from 0 up (the i-k-j loop).
#include <cstdio>
#include <vector>

int main() {
    const int n = 1024;
    std::vector<double> a(n * n), b(n * n);
    for (int i = 0; i < n * n; i++) {
        a[i] = (double)(i % 97) * 0.01;
        b[i] = (double)(i % 89) * 0.02 - 0.5;
    }
    double total = 0.0;
    for (int round = 0; round < 4; round++) {
        std::vector<double> c(n * n, 0.0);
        for (int i = 0; i < n; i++) {
            for (int k = 0; k < n; k++) {
                double x = a[i * n + k];
                for (int j = 0; j < n; j++) {
                    c[i * n + j] = c[i * n + j] + x * b[k * n + j];
                }
            }
        }
        total += c[round * 1000];
    }
    std::printf("%.17g\n", total);
}

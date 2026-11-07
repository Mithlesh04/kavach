// Nested lists: multiply two 300 x 300 matrices (27 million multiply-adds).
#include <cstdio>
#include <vector>

using Matrix = std::vector<std::vector<double>>;

Matrix make(long long n, long long seed) {
    Matrix rows;
    for (long long i = 0; i < n; i++) {
        std::vector<double> row;
        for (long long j = 0; j < n; j++) {
            row.push_back(double((i * seed + j) % 10));
        }
        rows.push_back(row);
    }
    return rows;
}

int main() {
    long long n = 300;
    Matrix a = make(n, 3);
    Matrix b = make(n, 7);
    double total = 0.0;
    for (long long i = 0; i < n; i++) {
        const std::vector<double>& row = a[i];
        for (long long j = 0; j < n; j++) {
            double cell = 0.0;
            for (long long k = 0; k < n; k++) {
                cell += row[k] * b[k][j];
            }
            total += cell;
        }
    }
    std::printf("%.1f\n", total);
}

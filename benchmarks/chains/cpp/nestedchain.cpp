// A chain inside a chain, as a C++ programmer writes it: two loops.
#include <cstdio>
#include <vector>

int main() {
    std::vector<std::vector<long long>> rows;
    for (long long r = 0; r < 10000; r++) {
        std::vector<long long> row(1000);
        for (long long i = 0; i < 1000; i++) {
            row[i] = (r + i) % 1000;
        }
        rows.push_back(row);
    }
    long long total = 0;
    for (const auto& row : rows) {
        for (long long x : row) {
            long long y = x * 2;
            if (y % 3 != 0) {
                total += y;
            }
        }
    }
    std::printf("%lld\n", total);
}

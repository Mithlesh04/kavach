// A chain over data, as a C++ programmer writes it: one loop, no
// intermediate vectors.
#include <cstdio>
#include <vector>

int main() {
    std::vector<long long> xs;
    for (long long i = 0; i < 10000000; i++) {
        xs.push_back(i % 1000);
    }
    long long total = 0;
    for (long long x : xs) {
        long long y = x * 3 + 1;
        if (y % 2 == 0) {
            total += y / 2;
        }
    }
    std::printf("%lld\n", total);
}

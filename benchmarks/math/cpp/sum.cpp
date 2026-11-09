// Summing a big list: 50 million integers, added up 10 times.
#include <cstdio>
#include <vector>

int main() {
    std::vector<long long> xs;
    for (long long i = 0; i < 50000000; i++) {
        xs.push_back(i % 1000);
    }
    long long total = 0;
    for (int round = 0; round < 10; round++) {
        for (long long x : xs) {
            total += x;
        }
    }
    std::printf("%lld\n", total);
}

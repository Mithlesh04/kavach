// A growing list: push 20 million numbers, then add them up on all
// cores. Whole numbers add up to the same total in any order.
// Build: g++ -O2 -fopenmp lists_cores.cpp
#include <cstdio>
#include <vector>

int main() {
    std::vector<long long> items;
    for (long long i = 0; i < 20000000; i++) {
        items.push_back(i % 1000);
    }
    long long total = 0;
    const long long n = (long long)items.size();
#pragma omp parallel for reduction(+ : total)
    for (long long k = 0; k < n; k++) {
        total += items[k];
    }
    std::printf("%lld\n", total);
}

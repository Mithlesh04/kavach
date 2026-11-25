// Sorting 5 million numbers on all cores. As Kavach's sort does, one
// pass splits the numbers by their top 8 bits into 256 buckets, then
// the buckets are sorted on all cores (here with std::sort). The
// numbers are never negative.
// Build: g++ -O2 -fopenmp sort_cores.cpp
#include <algorithm>
#include <cstdio>
#include <vector>

static void sort_on_cores(std::vector<long long>& v) {
    long long all = 0;
    for (long long x : v) all |= x;
    int bits = 64 - __builtin_clzll((unsigned long long)all | 1);
    int shift = bits > 8 ? bits - 8 : 0;
    std::vector<size_t> start(257, 0);
    for (long long x : v) start[(x >> shift) + 1]++;
    for (int b = 0; b < 256; b++) start[b + 1] += start[b];
    std::vector<long long> out(v.size());
    std::vector<size_t> at(start.begin(), start.end() - 1);
    for (long long x : v) out[at[x >> shift]++] = x;
#pragma omp parallel for schedule(dynamic)
    for (int b = 0; b < 256; b++) {
        std::sort(out.begin() + start[b], out.begin() + start[b + 1]);
    }
    v.swap(out);
}

int main() {
    std::vector<long long> items;
    long long x = 12345;
    for (long long i = 0; i < 5000000; i++) {
        x = (x * 1103515245 + 12345) % 2147483648;
        items.push_back(x);
    }
    std::vector<long long> sorted = items;
    sort_on_cores(sorted);
    std::printf("%lld\n", sorted[0] + sorted[4999999]);
}

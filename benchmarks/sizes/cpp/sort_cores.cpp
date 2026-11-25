// Sorting --items integers a Lehmer generator makes, on all cores. As
// Kavach's sort does, one pass splits the numbers by their top 8 bits
// into 256 buckets, then the buckets are sorted on all cores (here
// with std::sort). The numbers are never negative.
// Build: g++ -O2 -fopenmp sort_cores.cpp
#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

static void sort_on_cores(std::vector<int64_t>& v) {
    int64_t all = 0;
    for (int64_t x : v) all |= x;
    int bits = 64 - __builtin_clzll((uint64_t)all | 1);
    int shift = bits > 8 ? bits - 8 : 0;
    std::vector<size_t> start(257, 0);
    for (int64_t x : v) start[(x >> shift) + 1]++;
    for (int b = 0; b < 256; b++) start[b + 1] += start[b];
    std::vector<int64_t> out(v.size());
    std::vector<size_t> at(start.begin(), start.end() - 1);
    for (int64_t x : v) out[at[x >> shift]++] = x;
#pragma omp parallel for schedule(dynamic)
    for (int b = 0; b < 256; b++) {
        std::sort(out.begin() + start[b], out.begin() + start[b + 1]);
    }
    v.swap(out);
}

int main(int argc, char** argv) {
    int64_t n = 10'000'000;
    for (int a = 1; a + 1 < argc; a++) {
        if (std::strcmp(argv[a], "--items") == 0) n = std::atoll(argv[a + 1]);
    }
    std::vector<int64_t> xs;
    int64_t j = 12345;
    for (int64_t i = 0; i < n; i++) {
        j = j * 48271 % 2147483647;
        xs.push_back(j);
    }
    sort_on_cores(xs);
    printf("%lld\n", (long long)(xs[0] + xs[xs.size() / 2] + xs[xs.size() - 1]));
}

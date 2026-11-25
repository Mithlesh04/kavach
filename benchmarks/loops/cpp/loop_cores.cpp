// A tight `for` loop over integers: 500 million rounds, on all cores.
// Each thread keeps its own sum modulo 1,000,000,007, and the sums are
// added modulo the same number: no part is negative, so the result is
// the one-core loop's, as Kavach's `on cores` gives it.
// Build: g++ -O2 -fopenmp loop_cores.cpp
#include <cstdio>

#pragma omp declare reduction(modadd : long long : omp_out = (omp_out + omp_in) % 1000000007) \
    initializer(omp_priv = 0)

int main() {
    long long total = 0;
#pragma omp parallel for reduction(modadd : total)
    for (long long i = 0; i < 500000000; i++) {
        total = (total + i * 7 % 13) % 1000000007;
    }
    std::printf("%lld\n", total);
}

// Memory management: a short-lived list made, read and freed, 20 million
// times.
#include <cstdio>
#include <vector>

int main() {
    long long total = 0;
    for (long long i = 0; i < 20000000; i++) {
        std::vector<long long> xs = {i, i + 1, i + 2};
        total += xs[0] + xs[2];
    }
    std::printf("%lld\n", total);
}

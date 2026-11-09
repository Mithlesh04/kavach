// Single reads from a large list: 50 million lookups at indexes a
// Lehmer generator picks, over 10 million items (80 MB, past the caches).
#include <cstdint>
#include <cstdio>
#include <vector>

int main() {
    std::vector<int64_t> xs;
    for (int64_t i = 0; i < 10'000'000; i++) {
        xs.push_back(i * 2);
    }
    int64_t j = 12345;
    int64_t total = 0;
    for (int64_t r = 0; r < 50'000'000; r++) {
        j = j * 48271 % 2147483647;
        total += xs[j % 10'000'000];
    }
    printf("%lld\n", (long long)total);
}

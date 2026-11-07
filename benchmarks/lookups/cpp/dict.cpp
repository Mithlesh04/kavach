// Single reads from a large map: 20 million lookups at keys a Lehmer
// generator picks, over 2 million entries.
#include <cstdint>
#include <cstdio>
#include <unordered_map>

int main() {
    std::unordered_map<int64_t, int64_t> m;
    for (int64_t i = 0; i < 2'000'000; i++) {
        m[i] = i * 2 + 1;
    }
    int64_t j = 12345;
    int64_t total = 0;
    for (int64_t r = 0; r < 20'000'000; r++) {
        j = j * 48271 % 2147483647;
        auto it = m.find(j % 2'000'000);
        total += it == m.end() ? 0 : it->second;
    }
    printf("%lld\n", (long long)total);
}

// 50 million reads at indexes a Lehmer generator picks, from a list of
// --items integers: the same work at every size.
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>

int main(int argc, char** argv) {
    int64_t n = 10'000'000;
    for (int a = 1; a + 1 < argc; a++) {
        if (std::strcmp(argv[a], "--items") == 0) n = std::atoll(argv[a + 1]);
    }
    std::vector<int64_t> xs;
    for (int64_t i = 0; i < n; i++) {
        xs.push_back(i * 2);
    }
    int64_t j = 12345;
    int64_t total = 0;
    for (int64_t r = 0; r < 50'000'000; r++) {
        j = j * 48271 % 2147483647;
        total += xs[j % n];
    }
    printf("%lld\n", (long long)total);
}

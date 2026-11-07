// Sorting --items integers a Lehmer generator makes.
#include <algorithm>
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
    int64_t j = 12345;
    for (int64_t i = 0; i < n; i++) {
        j = j * 48271 % 2147483647;
        xs.push_back(j);
    }
    std::sort(xs.begin(), xs.end());
    printf("%lld\n", (long long)(xs[0] + xs[xs.size() / 2] + xs[xs.size() - 1]));
}

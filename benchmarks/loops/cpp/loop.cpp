// A tight `for` loop over integers: 500 million rounds.
#include <cstdio>

int main() {
    long long total = 0;
    for (long long i = 0; i < 500000000; i++) {
        total = (total + i * 7 % 13) % 1000000007;
    }
    std::printf("%lld\n", total);
}

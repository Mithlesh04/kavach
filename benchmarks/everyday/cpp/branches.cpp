// Conditions: an if/else chain decided 200 million times.
#include <cstdio>

int main() {
    long long total = 0;
    for (long long i = 0; i < 200000000; i++) {
        long long x = i % 7;
        if (x == 0) {
            total += 3;
        } else if (x < 3) {
            total += x;
        } else if (x == 5) {
            total -= 1;
        } else {
            total += 2;
        }
    }
    std::printf("%lld\n", total);
}

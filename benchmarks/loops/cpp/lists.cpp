// A growing list: push 20 million numbers, then add them up.
#include <cstdio>
#include <vector>

int main() {
    std::vector<long long> items;
    for (long long i = 0; i < 20000000; i++) {
        items.push_back(i % 1000);
    }
    long long total = 0;
    for (long long x : items) {
        total += x;
    }
    std::printf("%lld\n", total);
}

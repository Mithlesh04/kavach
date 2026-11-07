// Sorting 5 million numbers.
#include <algorithm>
#include <cstdio>
#include <vector>

int main() {
    std::vector<long long> items;
    long long x = 12345;
    for (long long i = 0; i < 5000000; i++) {
        x = (x * 1103515245 + 12345) % 2147483648;
        items.push_back(x);
    }
    std::vector<long long> sorted = items;
    std::sort(sorted.begin(), sorted.end());
    std::printf("%lld\n", sorted[0] + sorted[4999999]);
}

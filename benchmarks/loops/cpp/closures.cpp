// Lambdas over a big list: map then filter then sum, 10 million items.
#include <cstdio>
#include <vector>

int main() {
    std::vector<long long> items;
    for (long long i = 0; i < 10000000; i++) {
        items.push_back(i % 1000);
    }
    std::vector<long long> doubled;
    for (long long x : items) doubled.push_back(x * 2);
    std::vector<long long> big;
    for (long long x : doubled) if (x > 1000) big.push_back(x);
    long long total = 0;
    for (long long x : big) total += x;
    std::printf("%lld\n", total);
}

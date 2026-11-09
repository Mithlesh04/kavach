// A map with 2 million keys: insert, then look every key up.
#include <cstdio>
#include <unordered_map>

int main() {
    std::unordered_map<long long, long long> squares;
    for (long long i = 0; i < 2000000; i++) {
        squares[i] = i * i % 1000;
    }
    long long total = 0;
    for (long long i = 0; i < 2000000; i++) {
        auto found = squares.find(i);
        total += found == squares.end() ? 0 : found->second;
    }
    std::printf("%lld\n", total);
}

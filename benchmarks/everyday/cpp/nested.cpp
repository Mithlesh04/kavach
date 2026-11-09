// Nested callbacks: fifty handlers each wrapping the next, called two
// million times.
#include <cstdio>
#include <functional>

int main() {
    std::function<long long(long long)> f = [](long long x) { return x + 1; };
    for (int layer = 0; layer < 50; layer++) {
        auto inner = f;
        f = [inner](long long x) { return inner(x) + 1; };
    }
    long long total = 0;
    for (long long i = 0; i < 2000000; i++) {
        total += f(i % 100);
    }
    std::printf("%lld\n", total);
}

// A callback held in a record, called 100 million times.
#include <cstdio>
#include <functional>

struct Handler {
    std::function<long long(long long)> on;
};

static long long add_tax(long long n) {
    return n + n / 5;
}

int main() {
    Handler h{add_tax};
    long long total = 0;
    for (long long i = 0; i < 100000000; i++) {
        total += h.on(i % 1000);
    }
    std::printf("%lld\n", total);
}

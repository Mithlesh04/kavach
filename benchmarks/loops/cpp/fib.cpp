// Recursive calls: fib(38) makes about 126 million calls.
#include <cstdio>

long long fib(long long n) {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

int main() {
    std::printf("%lld\n", fib(38));
}

// The call stack: recursion 100 thousand frames deep, a thousand times.
#include <cstdio>

static long long down(long long n, long long salt) {
    if (n == 0) {
        return salt;
    }
    return (n + salt) % 3 + down(n - 1, salt);
}

int main() {
    long long total = 0;
    for (int round = 0; round < 1000; round++) {
        total += down(100000, round % 5);
    }
    std::printf("%lld\n", total);
}

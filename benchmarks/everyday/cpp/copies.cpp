// Assignment: a record copied, changed and read 100 million times.
#include <cstdio>

struct Point {
    long long x, y, z;
};

int main() {
    Point p{1, 2, 3};
    long long total = 0;
    for (long long i = 0; i < 100000000; i++) {
        Point q = p;
        q.x = i + (total & 1);
        total += q.x + q.z;
        p.y = total & 15;
    }
    std::printf("%lld\n", total);
}

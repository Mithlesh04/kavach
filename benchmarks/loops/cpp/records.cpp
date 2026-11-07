// Records: create 10 million points in a list, then read their fields.
#include <cstdio>
#include <vector>

struct Point {
    long long x;
    long long y;
};

int main() {
    std::vector<Point> points;
    for (long long i = 0; i < 10000000; i++) {
        points.push_back(Point{i % 100, i % 7});
    }
    long long total = 0;
    for (const Point& p : points) {
        total += p.x * p.y;
    }
    std::printf("%lld\n", total);
}

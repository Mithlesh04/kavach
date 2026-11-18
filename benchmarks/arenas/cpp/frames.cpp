// A temporary list each frame: 100 frames, each collecting the hits of
// 100,000 particles into a list of records, adding them up, then dropping it.
#include <cstdio>
#include <vector>

struct Hit {
    long long a, b;
    double d;
};

int main() {
    std::vector<double> xs;
    for (int i = 0; i < 100000; i++) {
        xs.push_back((double)(i % 1000) * 0.5);
    }
    double total = 0.0;
    for (int frame = 0; frame < 100; frame++) {
        std::vector<Hit> hits;
        for (int i = 0; i < 100000; i++) {
            double d = xs[i] - (double)frame;
            if (d > 0.0 && d < 200.0) {
                hits.push_back(Hit{i, frame, d});
            }
        }
        double s = 0.0;
        for (const auto& h : hits) {
            s += h.d;
        }
        total += s;
    }
    std::printf("%.1f\n", total);
}

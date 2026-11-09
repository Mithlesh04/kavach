// The dot product of two 20-million-item float lists, 10 times.
#include <cstdio>
#include <vector>

int main() {
    std::vector<double> xs, ys;
    for (long long i = 0; i < 20000000; i++) {
        double f = (double)(i % 1000);
        xs.push_back(f * 0.5);
        ys.push_back(f * 0.25 + 1.0);
    }
    double total = 0.0;
    for (int round = 0; round < 10; round++) {
        double dot = 0.0;
        for (long long i = 0; i < 20000000; i++) {
            dot += xs[i] * ys[i];
        }
        total += dot;
    }
    std::printf("%s\n", total > 0.0 ? "true" : "false");
}

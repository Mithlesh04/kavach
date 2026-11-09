// Element-wise arithmetic: y[i] = x[i] * 2.5 + 1.0 over 20 million
// floats, 10 times.
#include <cstdio>
#include <vector>

int main() {
    std::vector<double> xs, ys;
    for (long long i = 0; i < 20000000; i++) {
        xs.push_back((double)(i % 1000) * 0.125);
        ys.push_back(0.0);
    }
    for (int round = 0; round < 10; round++) {
        for (long long i = 0; i < 20000000; i++) {
            ys[i] = xs[i] * 2.5 + 1.0;
        }
    }
    std::printf("%s\n", ys[19999999] > 0.0 ? "true" : "false");
}

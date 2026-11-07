// Square roots over a big list: 20 million, 10 times.
#include <cmath>
#include <cstdio>
#include <vector>

int main() {
    std::vector<double> xs, ys;
    for (long long i = 0; i < 20000000; i++) {
        xs.push_back((double)(i % 1000) + 0.5);
        ys.push_back(0.0);
    }
    for (int round = 0; round < 10; round++) {
        for (long long i = 0; i < 20000000; i++) {
            ys[i] = std::sqrt(xs[i]);
        }
    }
    std::printf("%s\n", ys[1234] > 1.0 ? "true" : "false");
}

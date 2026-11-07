// The same work as kavach/glow.kav: 4 million floats, 1,000 multiply-adds
// each. Built single-threaded (-O2), and on all cores (-O3 -march=native
// -fopenmp), the best a CPU does.
#include <chrono>
#include <cstdio>
#include <vector>

static std::vector<float> glow(const std::vector<float>& xs, float gain) {
    std::vector<float> out(xs.size());
#pragma omp parallel for
    for (long i = 0; i < (long)xs.size(); i++) {
        float v = xs[i];
        for (int k = 0; k < 1000; k++) v = v * 0.999f + gain;
        out[i] = v;
    }
    return out;
}

int main() {
    std::vector<float> xs(4000000);
    for (long i = 0; i < 4000000; i++) xs[i] = float(i % 1000) * 0.001f;
    auto t0 = std::chrono::steady_clock::now();
    auto first = glow(xs, 0.25f);
    auto t1 = std::chrono::steady_clock::now();
    auto second = glow(xs, 0.75f);
    auto t2 = std::chrono::steady_clock::now();
    auto ms = [](auto a, auto b) { return (long)std::chrono::duration_cast<std::chrono::milliseconds>(b - a).count(); };
    std::printf("first map %ld ms, second map %ld ms\n%g %g\n", ms(t0, t1), ms(t1, t2), first[3999999], second[7]);
}

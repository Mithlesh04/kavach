// The game-style loop of kavach/frames.kav in C++: 1 million particles,
// each moved and bounced inside a box for 300 frames, one frame after
// another. With -fopenmp each frame runs on every core.
#include <algorithm>
#include <chrono>
#include <cstdio>
#include <vector>

struct Particle {
    float x, y, vx, vy;
};

static Particle step(Particle p) {
    const float dt = 0.016f;
    float vy = p.vy - 9.8f * dt;
    float x = p.x + p.vx * dt;
    float y = p.y + vy * dt;
    float vx = p.vx;
    if (x < 0.0f || x > 100.0f) {
        vx = 0.0f - vx;
        x = std::min(std::max(x, 0.0f), 100.0f);
    }
    if (y < 0.0f) {
        vy = 0.0f - vy * 0.9f;
        y = 0.0f;
    }
    return Particle{x, y, vx, vy};
}

int main() {
    const int n = 1000000;
    std::vector<Particle> ps(n);
    for (int i = 0; i < n; i++) {
        float f = float(i % 1000);
        ps[i] = Particle{f * 0.1f, 50.0f + f * 0.01f, f * 0.003f - 1.5f, 0.0f};
    }
    auto t0 = std::chrono::steady_clock::now();
    for (int frame = 0; frame < 300; frame++) {
#pragma omp parallel for
        for (int i = 0; i < n; i++) {
            ps[i] = step(ps[i]);
        }
    }
    auto t1 = std::chrono::steady_clock::now();
    long ms = std::chrono::duration_cast<std::chrono::milliseconds>(t1 - t0).count();
    std::printf("300 frames in %ld ms\n", ms);
    std::printf("%g %g\n", ps[n - 1].x, ps[n - 1].y);
}

// The game-style loop of kavach/frames.kav in CUDA: 1 million particles,
// each moved and bounced inside a box for 300 frames, one kernel launch
// per frame, the particles kept on the GPU. Prints the time to open the
// GPU apart from the frames (which include copying the particles over
// and the last frame back), as the Kavach run splits them.
#include <chrono>
#include <cstdio>
#include <vector>

struct Particle {
    float x, y, vx, vy;
};

__global__ void step(Particle* ps, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    Particle p = ps[i];
    const float dt = 0.016f;
    float vy = p.vy - 9.8f * dt;
    float x = p.x + p.vx * dt;
    float y = p.y + vy * dt;
    float vx = p.vx;
    if (x < 0.0f || x > 100.0f) {
        vx = 0.0f - vx;
        x = fminf(fmaxf(x, 0.0f), 100.0f);
    }
    if (y < 0.0f) {
        vy = 0.0f - vy * 0.9f;
        y = 0.0f;
    }
    ps[i] = Particle{x, y, vx, vy};
}

static long since(std::chrono::steady_clock::time_point t) {
    auto d = std::chrono::steady_clock::now() - t;
    return (long)std::chrono::duration_cast<std::chrono::milliseconds>(d).count();
}

int main() {
    const int n = 1000000;
    std::vector<Particle> ps(n);
    for (int i = 0; i < n; i++) {
        float f = float(i % 1000);
        ps[i] = Particle{f * 0.1f, 50.0f + f * 0.01f, f * 0.003f - 1.5f, 0.0f};
    }
    auto t0 = std::chrono::steady_clock::now();
    cudaFree(0);
    long open = since(t0);
    auto t1 = std::chrono::steady_clock::now();
    Particle* on_gpu;
    cudaMalloc(&on_gpu, n * sizeof(Particle));
    cudaMemcpy(on_gpu, ps.data(), n * sizeof(Particle), cudaMemcpyHostToDevice);
    for (int frame = 0; frame < 300; frame++) {
        step<<<(n + 255) / 256, 256>>>(on_gpu, n);
    }
    cudaMemcpy(ps.data(), on_gpu, n * sizeof(Particle), cudaMemcpyDeviceToHost);
    long frames = since(t1);
    cudaFree(on_gpu);
    std::printf("opening the GPU %ld ms, 300 frames in %ld ms\n", open, frames);
    std::printf("%g %g\n", ps[n - 1].x, ps[n - 1].y);
}

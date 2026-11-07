// The loop of kavach/stream.kav in CUDA, written by hand as NVIDIA's
// examples do: a ring of slots, each with pinned memory and a stream of its
// own, so the next chunk is read and copied over while the GPU works on the
// last. Each chunk of 32-bit numbers goes through `work`, then is summed one
// block at a time. With `slots` 1 each chunk waits for the one before it.
//
//   stream <file> <steps> <slots>
//
// No check for overflow: the sums here are 64 bits, and the Kavach kernel
// checks each 32-bit step.
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <vector>

static const size_t CHUNK = 67108864;
static const int BLOCK = 256;

__global__ void work(const int* in, int* out, int n, int steps) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    int v = in[i];
    for (int s = 0; s < steps; s++) {
        v = (v * 3 + 1) % 100;
    }
    out[i] = v;
}

__global__ void sum_blocks(const int* xs, long long* sums, int n) {
    int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        atomicAdd((unsigned long long*)&sums[blockIdx.x], (unsigned long long)(long long)xs[i]);
    }
}

struct Slot {
    cudaStream_t stream;
    int* pinned;
    int* in;
    int* out;
    long long* sums;
    long long* sums_back;
    int n;
};

static long long collect(Slot& s) {
    long long total = 0;
    if (s.n > 0) {
        cudaStreamSynchronize(s.stream);
        int blocks = (s.n + BLOCK - 1) / BLOCK;
        for (int b = 0; b < blocks; b++) {
            total += s.sums_back[b];
        }
    }
    s.n = 0;
    return total;
}

int main(int argc, char** argv) {
    if (argc < 4) {
        std::fprintf(stderr, "stream <file> <steps> <slots>\n");
        return 2;
    }
    const int steps = std::atoi(argv[2]);
    const int count = std::atoi(argv[3]);
    FILE* file = std::fopen(argv[1], "rb");
    if (!file) {
        return 2;
    }
    const int blocks = (int)(CHUNK / 4 / BLOCK) + 1;
    std::vector<Slot> slots(count);
    for (Slot& s : slots) {
        cudaStreamCreate(&s.stream);
        cudaMallocHost(&s.pinned, CHUNK);
        cudaMalloc(&s.in, CHUNK);
        cudaMalloc(&s.out, CHUNK);
        cudaMalloc(&s.sums, blocks * sizeof(long long));
        cudaMallocHost(&s.sums_back, blocks * sizeof(long long));
        s.n = 0;
    }
    auto t0 = std::chrono::steady_clock::now();
    long long total = 0;
    for (int next = 0;; next = (next + 1) % count) {
        Slot& s = slots[next];
        total += collect(s);
        size_t got = std::fread(s.pinned, 1, CHUNK, file);
        if (got == 0) {
            break;
        }
        s.n = (int)(got / 4);
        int chunk_blocks = (s.n + BLOCK - 1) / BLOCK;
        cudaMemcpyAsync(s.in, s.pinned, got, cudaMemcpyHostToDevice, s.stream);
        cudaMemsetAsync(s.sums, 0, chunk_blocks * sizeof(long long), s.stream);
        work<<<chunk_blocks, BLOCK, 0, s.stream>>>(s.in, s.out, s.n, steps);
        sum_blocks<<<chunk_blocks, BLOCK, 0, s.stream>>>(s.out, s.sums, s.n);
        cudaMemcpyAsync(s.sums_back, s.sums, chunk_blocks * sizeof(long long),
                        cudaMemcpyDeviceToHost, s.stream);
    }
    for (Slot& s : slots) {
        total += collect(s);
    }
    auto d = std::chrono::steady_clock::now() - t0;
    long ms = (long)std::chrono::duration_cast<std::chrono::milliseconds>(d).count();
    std::printf("sum %lld, %ld ms\n", total, ms);
    std::fclose(file);
}

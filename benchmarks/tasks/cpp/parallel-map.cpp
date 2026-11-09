// Eight sums of 150 million rounds each: one by one, then one thread each.
#include <chrono>
#include <cstdio>
#include <thread>
#include <vector>

static long long work(long long seed) {
    long long total = 0;
    for (long long i = 0; i < 150000000; i++) {
        total = (total + i * seed) % 1000000007;
    }
    return total;
}

static long long now_ms() {
    using namespace std::chrono;
    return duration_cast<milliseconds>(steady_clock::now().time_since_epoch()).count();
}

int main() {
    const long long seeds[8] = {1, 2, 3, 4, 5, 6, 7, 8};
    long long one_by_one[8], parallel[8];
    long long t0 = now_ms();
    for (int k = 0; k < 8; k++) {
        one_by_one[k] = work(seeds[k]);
    }
    long long t1 = now_ms();
    std::vector<std::thread> threads;
    for (int k = 0; k < 8; k++) {
        threads.emplace_back([&, k] { parallel[k] = work(seeds[k]); });
    }
    for (auto &t : threads) {
        t.join();
    }
    long long t2 = now_ms();
    bool same = true;
    for (int k = 0; k < 8; k++) {
        same = same && one_by_one[k] == parallel[k];
    }
    std::printf("%s\n", same ? "true" : "false");
    std::printf("one by one %lld ms, parallel_map %lld ms\n", t1 - t0, t2 - t1);
}

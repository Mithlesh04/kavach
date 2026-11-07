// One thread sends 2 million integers through a bounded queue that holds
// 1024 (C++ has no channel: a mutex and two condition variables, as
// people write it); main receives them and adds them up.
#include <condition_variable>
#include <cstdio>
#include <deque>
#include <mutex>
#include <optional>
#include <thread>

struct Channel {
    std::mutex m;
    std::condition_variable not_full, not_empty;
    std::deque<long long> items;
    size_t capacity;
    bool closed = false;

    void put(long long v) {
        std::unique_lock<std::mutex> lock(m);
        not_full.wait(lock, [&] { return items.size() < capacity; });
        items.push_back(v);
        not_empty.notify_one();
    }

    void close() {
        std::lock_guard<std::mutex> lock(m);
        closed = true;
        not_empty.notify_all();
    }

    std::optional<long long> take() {
        std::unique_lock<std::mutex> lock(m);
        not_empty.wait(lock, [&] { return !items.empty() || closed; });
        if (items.empty()) return std::nullopt;
        long long v = items.front();
        items.pop_front();
        not_full.notify_one();
        return v;
    }
};

int main() {
    Channel ch;
    ch.capacity = 1024;
    std::thread producer([&] {
        for (long long i = 0; i < 2000000; i++) ch.put(i);
        ch.close();
    });
    long long total = 0;
    while (auto v = ch.take()) total += *v;
    producer.join();
    std::printf("%lld\n", total);
}

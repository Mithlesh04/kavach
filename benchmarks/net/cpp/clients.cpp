// An echo server with 100 clients at once, each making 100 round trips
// of 4 bytes over TCP on this machine: a thread per connection on both
// sides, with blocking sockets, as most C++ servers are first written.
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <unistd.h>

#include <atomic>
#include <cstdio>
#include <thread>
#include <vector>

static void nodelay(int fd) {
    int one = 1;
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &one, sizeof one);
}

int main() {
    const int clients = 100, trips = 100;
    int listener = socket(AF_INET, SOCK_STREAM, 0);
    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    bind(listener, (sockaddr*)&addr, sizeof addr);
    listen(listener, 256);
    socklen_t len = sizeof addr;
    getsockname(listener, (sockaddr*)&addr, &len);

    std::atomic<long> served{0};
    std::thread server([&] {
        std::vector<std::thread> handlers;
        for (int i = 0; i < clients; i++) {
            int conn = accept(listener, nullptr, nullptr);
            nodelay(conn);
            handlers.emplace_back([conn, &served] {
                char buf[64];
                ssize_t n;
                while ((n = read(conn, buf, sizeof buf)) > 0) {
                    if (write(conn, buf, n) != n) break;
                    served++;
                }
                close(conn);
            });
        }
        for (auto& h : handlers) h.join();
    });

    std::atomic<long> back{0};
    std::vector<std::thread> users;
    for (int c = 0; c < clients; c++) {
        users.emplace_back([&] {
            int conn = socket(AF_INET, SOCK_STREAM, 0);
            nodelay(conn);
            connect(conn, (sockaddr*)&addr, sizeof addr);
            char buf[64];
            for (int i = 0; i < trips; i++) {
                if (write(conn, "ping", 4) != 4) break;
                back += read(conn, buf, sizeof buf);
            }
            close(conn);
        });
    }
    for (auto& u : users) u.join();
    server.join();
    printf("%ld trips, %ld bytes back\n", served.load(), back.load());
}

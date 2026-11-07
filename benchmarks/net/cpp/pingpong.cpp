// 10,000 round trips over TCP on this machine: a client thread sends 8
// bytes, the server thread sends them back. Blocking sockets, as most
// C++ programs use them.
#include <arpa/inet.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <sys/socket.h>
#include <unistd.h>

#include <cstdio>
#include <thread>

int main() {
    int listener = socket(AF_INET, SOCK_STREAM, 0);
    sockaddr_in addr{};
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    addr.sin_port = 0;
    bind(listener, (sockaddr*)&addr, sizeof addr);
    listen(listener, 16);
    socklen_t len = sizeof addr;
    getsockname(listener, (sockaddr*)&addr, &len);

    int trips = 0;
    std::thread server([&] {
        int conn = accept(listener, nullptr, nullptr);
        int one = 1;
        setsockopt(conn, IPPROTO_TCP, TCP_NODELAY, &one, sizeof one);
        char buf[64];
        ssize_t n;
        while ((n = read(conn, buf, sizeof buf)) > 0) {
            write(conn, buf, n);
            trips++;
        }
        close(conn);
    });

    int conn = socket(AF_INET, SOCK_STREAM, 0);
    int one = 1;
    setsockopt(conn, IPPROTO_TCP, TCP_NODELAY, &one, sizeof one);
    connect(conn, (sockaddr*)&addr, sizeof addr);
    char buf[64];
    long back = 0;
    for (int i = 0; i < 10000; i++) {
        write(conn, "pingpong", 8);
        back += read(conn, buf, sizeof buf);
    }
    close(conn);
    server.join();
    printf("%d trips, %ld bytes back\n", trips, back);
}

# An echo server with 100 clients at once, each making 100 round trips
# of 4 bytes over TCP on this machine: a thread per connection on both
# sides, with blocking sockets.
import socket
import threading

CLIENTS, TRIPS = 100, 100
listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(256)
served = 0
lock = threading.Lock()


def echo(conn):
    global served
    conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    while data := conn.recv(64):
        conn.sendall(data)
        with lock:
            served += 1
    conn.close()


def serve():
    handlers = []
    for _ in range(CLIENTS):
        conn, _ = listener.accept()
        h = threading.Thread(target=echo, args=(conn,))
        h.start()
        handlers.append(h)
    for h in handlers:
        h.join()


back = 0


def user():
    global back
    conn = socket.create_connection(listener.getsockname())
    conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    got = 0
    for _ in range(TRIPS):
        conn.sendall(b"ping")
        got += len(conn.recv(64))
    conn.close()
    with lock:
        back += got


server = threading.Thread(target=serve)
server.start()
users = [threading.Thread(target=user) for _ in range(CLIENTS)]
for u in users:
    u.start()
for u in users:
    u.join()
server.join()
print(f"{served} trips, {back} bytes back")

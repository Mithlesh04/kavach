# 10,000 round trips over TCP on this machine: a client sends 8 bytes,
# a server thread sends them back. Blocking sockets.
import socket
import threading

listener = socket.socket()
listener.bind(("127.0.0.1", 0))
listener.listen(16)
trips = 0


def serve():
    global trips
    conn, _ = listener.accept()
    conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
    while data := conn.recv(64):
        conn.sendall(data)
        trips += 1
    conn.close()


server = threading.Thread(target=serve)
server.start()
conn = socket.create_connection(listener.getsockname())
conn.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
back = 0
for _ in range(10_000):
    conn.sendall(b"pingpong")
    back += len(conn.recv(64))
conn.close()
server.join()
print(f"{trips} trips, {back} bytes back")

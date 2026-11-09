# One thread sends 2 million integers through a queue that holds 1024;
# the main thread receives them and adds them up.
import queue
import threading

N = 2_000_000
q = queue.Queue(maxsize=1024)
DONE = object()


def produce():
    for i in range(N):
        q.put(i)
    q.put(DONE)


producer = threading.Thread(target=produce)
producer.start()
total = 0
while True:
    v = q.get()
    if v is DONE:
        break
    total += v
producer.join()
print(total)

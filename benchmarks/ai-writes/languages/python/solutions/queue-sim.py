from collections import deque


def simulate(commands: list[int]) -> list[int]:
    queue: deque[int] = deque()
    for c in commands:
        if c > 0:
            queue.append(c)
        elif c == 0 and queue:
            queue.popleft()
    return list(queue)

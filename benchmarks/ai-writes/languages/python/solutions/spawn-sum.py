from concurrent.futures import ThreadPoolExecutor


def part(start: int, stop: int) -> int:
    return sum(k * k for k in range(start, stop))


def squares(n: int) -> int:
    mid = n // 2 + 1
    with ThreadPoolExecutor(max_workers=2) as pool:
        a = pool.submit(part, 1, mid)
        b = pool.submit(part, mid, n + 1)
        return a.result() + b.result()

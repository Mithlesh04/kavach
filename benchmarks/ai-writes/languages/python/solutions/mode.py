from collections import Counter


def mode(xs: list[int]) -> int:
    counts = Counter(xs)
    return min(counts, key=lambda x: (-counts[x], x))

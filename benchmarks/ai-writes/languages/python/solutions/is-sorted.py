def is_sorted(xs: list[int]) -> bool:
    return all(a <= b for a, b in zip(xs, xs[1:]))

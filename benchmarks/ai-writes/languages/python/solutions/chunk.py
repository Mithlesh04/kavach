def chunks(xs: list[int], n: int) -> list[list[int]]:
    return [xs[i:i + n] for i in range(0, len(xs), n)]

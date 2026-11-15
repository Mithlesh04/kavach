def clamp_all(xs: list[int], lo: int, hi: int) -> list[int]:
    return [min(max(x, lo), hi) for x in xs]

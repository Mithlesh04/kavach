def swap_ends(xs: list[int]) -> None:
    if len(xs) >= 2:
        xs[0], xs[-1] = xs[-1], xs[0]

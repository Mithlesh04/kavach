def first_negative(xs: list[int]) -> int | None:
    return next((i for i, x in enumerate(xs) if x < 0), None)

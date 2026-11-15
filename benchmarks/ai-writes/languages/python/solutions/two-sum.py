def two_sum(xs: list[int], target: int) -> tuple[int, int] | None:
    for i in range(len(xs)):
        for j in range(i + 1, len(xs)):
            if xs[i] + xs[j] == target:
                return (i, j)
    return None

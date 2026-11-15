def second_largest(xs: list[int]) -> int | None:
    distinct = sorted(set(xs), reverse=True)
    return distinct[1] if len(distinct) > 1 else None

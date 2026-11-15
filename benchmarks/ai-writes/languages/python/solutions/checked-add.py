def add_or_none(a: int, b: int) -> int | None:
    total = a + b
    if -(2**63) <= total <= 2**63 - 1:
        return total
    return None

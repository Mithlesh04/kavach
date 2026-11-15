def dedupe(xs: list[int]) -> list[int]:
    return list(dict.fromkeys(xs))

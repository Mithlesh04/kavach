def flatten(xss: list[list[int]]) -> list[int]:
    return [x for xs in xss for x in xs]

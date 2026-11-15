def transpose(m: list[list[int]]) -> list[list[int]]:
    return [list(column) for column in zip(*m)]

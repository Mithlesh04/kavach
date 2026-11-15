def pascal(n: int) -> list[int]:
    row = [1]
    for _ in range(n):
        row = [a + b for a, b in zip([0] + row, row + [0])]
    return row

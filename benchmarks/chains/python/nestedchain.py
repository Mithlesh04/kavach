# A chain inside a chain: ten thousand rows, each with its own chain.
rows = [[(r + i) % 1000 for i in range(1000)] for r in range(10_000)]
total = sum(
    sum(x for x in (x * 2 for x in row) if x % 3 != 0)
    for row in rows
)
print(total)

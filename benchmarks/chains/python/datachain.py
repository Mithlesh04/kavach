# A chain over data: map, filter, map, sum across 10 million items.
xs = [i % 1000 for i in range(10_000_000)]
total = sum(
    x // 2
    for x in (x * 3 + 1 for x in xs)
    if x % 2 == 0
)
print(total)

# Single reads from a large list: 50 million lookups at indexes a
# Lehmer generator picks, over 10 million items.


def main():
    xs = []
    for i in range(10_000_000):
        xs.append(i * 2)
    j = 12345
    total = 0
    for _ in range(50_000_000):
        j = j * 48271 % 2147483647
        total += xs[j % 10_000_000]
    print(total)


main()

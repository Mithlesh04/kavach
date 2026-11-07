# Single reads from a large map: 20 million lookups at keys a Lehmer
# generator picks, over 2 million entries.


def main():
    m = {}
    for i in range(2_000_000):
        m[i] = i * 2 + 1
    j = 12345
    total = 0
    for _ in range(20_000_000):
        j = j * 48271 % 2147483647
        total += m.get(j % 2_000_000, 0)
    print(total)


main()

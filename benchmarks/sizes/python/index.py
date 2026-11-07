# 50 million reads at indexes a Lehmer generator picks, from a list of
# --items integers: the same work at every size.
import sys


def main():
    n = int(sys.argv[sys.argv.index("--items") + 1]) if "--items" in sys.argv else 10_000_000
    xs = []
    for i in range(n):
        xs.append(i * 2)
    j = 12345
    total = 0
    for _ in range(50_000_000):
        j = j * 48271 % 2147483647
        total += xs[j % n]
    print(total)


main()

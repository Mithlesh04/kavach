# Sorting --items integers a Lehmer generator makes.
import sys


def main():
    n = int(sys.argv[sys.argv.index("--items") + 1]) if "--items" in sys.argv else 10_000_000
    xs = []
    j = 12345
    for _ in range(n):
        j = j * 48271 % 2147483647
        xs.append(j)
    s = sorted(xs)
    print(s[0] + s[len(s) // 2] + s[-1])


main()

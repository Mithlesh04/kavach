def primes_below(n: int) -> list[int]:
    if n < 3:
        return []
    sieve = [True] * n
    sieve[0] = sieve[1] = False
    for k in range(2, int(n**0.5) + 1):
        if sieve[k]:
            for m in range(k * k, n, k):
                sieve[m] = False
    return [k for k in range(n) if sieve[k]]

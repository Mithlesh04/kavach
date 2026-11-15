def factorial(n: int) -> int:
    result = 1
    for k in range(2, n + 1):
        result *= k
    return result

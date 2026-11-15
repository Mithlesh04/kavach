def sum_digits(text: str) -> int:
    return sum(int(c) for c in text if "0" <= c <= "9")

from collections import Counter


def anagrams(a: str, b: str) -> bool:
    return Counter(a.lower()) == Counter(b.lower())

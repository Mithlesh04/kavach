def count_vowels(text: str) -> int:
    return sum(1 for c in text.lower() if c in "aeiou")

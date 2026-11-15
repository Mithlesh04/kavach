def strip_vowels(text: str) -> str:
    return "".join(c for c in text if c.lower() not in "aeiou")

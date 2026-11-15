def is_palindrome(text: str) -> bool:
    lower = text.lower()
    return lower == lower[::-1]

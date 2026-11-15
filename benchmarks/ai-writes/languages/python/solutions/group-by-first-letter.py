def by_letter(words: list[str]) -> dict[str, list[str]]:
    groups: dict[str, list[str]] = {}
    for word in words:
        groups.setdefault(word[0], []).append(word)
    return groups

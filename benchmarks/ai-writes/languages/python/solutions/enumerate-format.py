def numbered(items: list[str]) -> str:
    return "\n".join(f"{i}. {item}" for i, item in enumerate(items, 1))

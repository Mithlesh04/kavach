def caesar(text: str, k: int) -> str:
    out = []
    for c in text:
        if "a" <= c <= "z":
            c = chr((ord(c) - ord("a") + k) % 26 + ord("a"))
        out.append(c)
    return "".join(out)

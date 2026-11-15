def csv(n: int) -> str:
    text = ""
    for k in range(1, n + 1):
        if text:
            text += ","
        text += str(k)
    return text

from itertools import groupby


def rle(text: str) -> str:
    return "".join(f"{c}{len(list(run))}" for c, run in groupby(text))

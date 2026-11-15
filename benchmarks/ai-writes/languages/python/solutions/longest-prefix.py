import os


def common_prefix(words: list[str]) -> str:
    return os.path.commonprefix(words)

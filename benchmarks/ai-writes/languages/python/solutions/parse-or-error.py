class BadInput(Exception):
    pass


class NotANumber(BadInput):
    def __init__(self, text: str):
        super().__init__(text)
        self.text = text


def doubled(text: str) -> int:
    try:
        n = int(text)
    except ValueError:
        raise NotANumber(text) from None
    return n * 2

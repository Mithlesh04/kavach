class MathError(Exception):
    pass


class DivideByZero(MathError):
    pass


def divide(a: int, b: int) -> int:
    if b == 0:
        raise DivideByZero()
    return a // b

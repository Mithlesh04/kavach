from decimal import Decimal


def total(items: list[tuple[Decimal, int]]) -> Decimal:
    return sum((price * qty for price, qty in items), Decimal(0))

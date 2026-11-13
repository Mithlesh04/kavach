from dataclasses import dataclass


@dataclass
class Order:
    item: str
    qty: int
    cents: int

    def cost(self) -> int:
        return self.qty * self.cents


class BadQty(Exception):
    def __init__(self, item: str):
        self.item = item


def total(orders: list[Order]) -> int:
    for o in orders:
        if o.qty <= 0:
            raise BadQty(o.item)
    return sum(o.cost() for o in orders)


def main():
    codes = {"SPRING": 10, "VIP": 25}
    orders = [Order("pen", 3, 150), Order("cup", 1, 900)]

    try:
        total_ = total(orders)
    except BadQty:
        total_ = 0
    off = codes.get("SPRING", 0)
    print(f"total {total_}, with {off}% off {total_ * (100 - off) // 100}")
    print(f"WINTER gives {codes.get('WINTER', 0)}%")

    broken = [Order("kite", 0, 500)]
    try:
        print(f"total {total(broken)}")
    except BadQty as e:
        print(f"refused: no quantity for {e.item}")


main()

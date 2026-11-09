# A chain of builder methods, 10 million times.
from dataclasses import replace, dataclass


@dataclass(frozen=True)
class Transfer:
    sender: int = 0
    to: int = 0
    amount: int = 0
    fee: int = 0
    note: int = 0

    def from_account(self, a):
        return replace(self, sender=a)

    def to_account(self, a):
        return replace(self, to=a)

    def pay(self, n):
        return replace(self, amount=n)

    def with_fee(self, n):
        return replace(self, fee=n)

    def tagged(self, n):
        return replace(self, note=n)


total = 0
for i in range(10_000_000):
    t = (
        Transfer()
        .from_account(i)
        .to_account(i + 1)
        .pay(i % 100)
        .with_fee(i % 7)
        .tagged(i % 3)
    )
    total += t.amount + t.fee + t.note
print(total)

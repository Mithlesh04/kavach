from dataclasses import dataclass


@dataclass
class Account:
    owner: str
    balance: int


from dataclasses import replace


def deposit(a: Account, amount: int) -> Account:
    return replace(a, balance=a.balance + amount)

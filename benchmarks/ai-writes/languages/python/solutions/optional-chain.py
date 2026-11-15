from dataclasses import dataclass


@dataclass
class Address:
    city: str | None


@dataclass
class User:
    address: Address | None


def city(u: User) -> str:
    if u.address is None or u.address.city is None:
        return "UNKNOWN"
    return u.address.city.upper()

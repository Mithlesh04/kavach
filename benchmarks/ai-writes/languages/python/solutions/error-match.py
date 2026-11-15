from dataclasses import dataclass


@dataclass
class NoSuchUser:
    name: str


@dataclass
class WrongPassword:
    pass


@dataclass
class Locked:
    minutes: int


LoginError = NoSuchUser | WrongPassword | Locked


def message(e: LoginError) -> str:
    match e:
        case NoSuchUser(name=name):
            return f"no user {name}"
        case WrongPassword():
            return "wrong password"
        case Locked(minutes=minutes):
            return f"locked for {minutes} minutes"

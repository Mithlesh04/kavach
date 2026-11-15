from abc import ABC, abstractmethod


class Named(ABC):
    @abstractmethod
    def name(self) -> str: ...


class Dog(Named):
    def name(self) -> str:
        return "dog"


class Cat(Named):
    def name(self) -> str:
        return "cat"


def names(xs: list[Named]) -> str:
    return ", ".join(x.name() for x in xs)

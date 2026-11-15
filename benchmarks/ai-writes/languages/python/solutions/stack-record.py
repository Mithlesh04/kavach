from dataclasses import dataclass


@dataclass
class Stack:
    items: list[int]

    def push(self, x: int) -> "Stack":
        return Stack(items=self.items + [x])

    def pop(self) -> "Stack":
        if not self.items:
            return self
        return Stack(items=self.items[:-1])

    def top(self) -> int | None:
        return self.items[-1] if self.items else None

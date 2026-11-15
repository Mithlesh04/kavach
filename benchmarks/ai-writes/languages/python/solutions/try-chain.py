class PortError(Exception):
    pass


class Bad(PortError):
    def __init__(self, text: str):
        super().__init__(text)
        self.text = text


def parse_port(text: str) -> int:
    try:
        return int(text)
    except ValueError:
        raise Bad(text) from None


def port_sum(text: str) -> int:
    parts = text.split(":")
    if len(parts) != 2:
        raise Bad(text)
    return parse_port(parts[0]) + parse_port(parts[1])

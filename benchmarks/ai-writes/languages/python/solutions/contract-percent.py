def percent(part: int, whole: int) -> int:
    assert whole > 0
    assert 0 <= part <= whole
    return part * 100 // whole

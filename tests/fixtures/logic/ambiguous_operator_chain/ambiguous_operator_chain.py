def chains(key, value, table, low, high, first, second, third, frame):
    return (
        key == value in table,
        key in table in value,
        0 < low is None,
        low < value > high,
        low == value is None,
        first != second != third,
        frame.a > 1 & frame.b < 2,
        frame.a > (1 & frame.b) < 2,
        low <= value < high,
        high >= value > low,
        first == second == third,
        first == second != 0,
        first is second is None,
        value & 1 == 0,
        value == low | high,
        value > 1 & high,
    )

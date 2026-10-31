VALUE = 1
counter = 0


def bump() -> None:
    global counter
    counter += VALUE


shared_value = 1

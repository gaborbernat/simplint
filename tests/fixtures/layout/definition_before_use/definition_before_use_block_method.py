def _label(value: int) -> str:
    return str(value)


if __name__ == "__main__":

    class Printer:
        def show(self) -> str:
            return _label(1)

    Printer().show()


class Shown:
    if DEBUG:

        def show(self) -> str:
            return _label(2)

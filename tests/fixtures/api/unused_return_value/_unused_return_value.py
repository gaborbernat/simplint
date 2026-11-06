def ret_dropped(value: int) -> int:
    return value + 1


def ret_read(value: int) -> int:
    return value + 2


def ret_tests_only(value: int) -> int:
    return value + 3


def ret_none(value: int) -> None:
    print(value)


def ret_uncalled(value: int) -> int:
    return value


def ret_passed(value: int) -> int:
    return value + 4


class RetWriter:
    def ret_write_rows(self, data: str) -> int:
        return len(data)

    def ret_flush_rows(self) -> int:
        return 0

    def ret_commit(self, data: str) -> int:
        self.ret_write_rows(data)
        return self.ret_flush_rows()


def ret_tests_read(value: int) -> int:
    return value + 5


class RetBuilder:
    def ret_add(self, item: str) -> "RetBuilder":
        print(item)
        return self

    def ret_build(self) -> None:
        self.ret_add("a")


def ret_exit_code() -> int:
    return 0


if __name__ == "__main__":
    ret_exit_code()

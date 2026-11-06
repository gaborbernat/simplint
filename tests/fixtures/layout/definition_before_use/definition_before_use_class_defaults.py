import itertools

counter = itertools.count()


class Counted:
    def _token(self, token: int = next(counter)) -> int:
        return token

    first = next(counter)

    def run(self) -> int:
        return self._token()


class Plain:
    def _token(self, token: int = 0) -> int:
        return token

    first = 1

    def run(self) -> int:
        return self._token()

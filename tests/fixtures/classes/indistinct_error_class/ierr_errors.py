from json import JSONDecodeError


class _IerrBase(Exception):
    pass


class _IerrCaught(_IerrBase):
    pass


class _IerrUnused(Exception):
    pass


class _IerrDerived(ValueError):
    pass


class _IerrBroad(BaseException):
    pass


class _IerrJson(JSONDecodeError):
    pass


class _IerrWarning(UserWarning):
    pass


class _IerrStateful(Exception):
    def __init__(self, code: int) -> None:
        super().__init__(code)


class _IerrDeclared(Exception):
    code = 1


class _IerrNotException:
    pass


class _IerrOwn(Exception):
    pass


class _IerrRelayed(Exception):
    pass


class _IerrChecked(Exception):
    pass


class _IerrMatched(Exception):
    pass


class _IerrStarred(Exception):
    pass


def guard() -> None:
    try:
        raise _IerrOwn()
    except _IerrOwn:
        pass


__all__ = []

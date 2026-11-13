import pickle


def _udm_outer(values: list[int]) -> list[int]:
    def _udm_unused_helper(value: int) -> int:
        return _udm_unused_helper(value - 1)

    def _udm_used_helper(value: int) -> int:
        return value * 2

    return [_udm_used_helper(value) for value in values]


def _udm_source_only() -> None:
    def _udm_written_out() -> str:
        return "plugin"


def _udm_dynamic() -> dict[str, object]:
    def _udm_by_locals() -> int:
        return 1

    return locals()


def _udm_deleted() -> None:
    def _udm_dropped() -> int:
        return 1

    del _udm_dropped


class _UdmCounter:
    def __init__(self, start: int) -> None:
        self._udm_start = start
        self._udm_never_read = start
        self._udm_bumped = 0
        self._udm_named_in_getattr = 1
        if start:
            self._udm_set_twice = 1
        self._udm_set_twice = 2

    def bump(self) -> int:
        self._udm_bumped += 1
        return self._udm_start + getattr(self, "_udm_named_in_getattr")


class _UdmPickled:
    def __init__(self) -> None:
        self._udm_kept_whole = 1

    def __getstate__(self) -> dict[str, int]:
        return {}


_UDM_STATE = pickle.dumps(_UdmPickled())
_UDM_TOTAL = _udm_outer([1]), _udm_source_only(), _udm_dynamic(), _UdmCounter(1).bump()

class _UdmDict:
    def __init__(self) -> None:
        self._udm_in_dict = 1

    def fields(self) -> dict[str, int]:
        return self.__dict__


_UDM_FIELDS = _UdmDict().fields()

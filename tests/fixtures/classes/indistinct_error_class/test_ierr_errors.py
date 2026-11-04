from ierr_errors import _IerrDerived, _IerrJson, _IerrUnused


def test_errors() -> None:
    try:
        raise _IerrUnused()
    except _IerrUnused:
        pass
    try:
        raise _IerrDerived()
    except _IerrDerived:
        pass
    try:
        raise _IerrJson()
    except _IerrJson:
        pass

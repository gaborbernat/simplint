import ierr_loop_a
from ierr_errors import _IerrBase, _IerrChecked, _IerrMatched, _IerrStarred
from ierr_reexport import _IerrRelayed


def run() -> None:
    try:
        raise _IerrBase()
    except (ValueError, _IerrBase):
        pass
    try:
        pass
    except ierr_loop_a.Missing:
        pass
    try:
        pass
    except errors.Unbound:
        pass
    try:
        raise _IerrRelayed()
    except _IerrRelayed:
        pass
    try:
        raise _IerrChecked()
    except Exception as error:
        if isinstance(error, _IerrChecked):
            pass
    try:
        raise _IerrMatched()
    except Exception as error:
        match error:
            case _IerrMatched():
                pass
    try:
        raise _IerrStarred()
    except* _IerrStarred:
        pass

from _test_only_parameter import connect, dispatch, retry, send

HANDLERS = [dispatch]
RETRY_ARGS = (1,)


def run() -> str:
    return send("x") + str(retry(1, delay=2.0)) + str(retry(*RETRY_ARGS)) + connect("db")

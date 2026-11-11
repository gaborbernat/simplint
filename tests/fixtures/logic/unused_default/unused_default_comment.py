def _connect(host, timeout=(  # seconds, as the gateway counts them
    30
)):
    return host, timeout


def _open_port(host, port=(  # the default port
    80
)):
    return host, port


def _retry(count=3):
    return count


_connect("db", 5)
_open_port("db", 8080)
_retry(4)

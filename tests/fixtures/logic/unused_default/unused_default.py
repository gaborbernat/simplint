import functools


def _connect(host, timeout=30):
    return host, timeout


def _retry(count=3, *, delay=1):
    return count, delay


def _fetch(url, retries=2, timeout=5):
    return url, retries, timeout


def _pair(first=1, second=2):
    return first, second


def _unused(value=1):
    return value


def _spread(value=1):
    return value


def _mapping(value=1):
    return value


def _passed_as_value(value=1):
    return value


@functools.cache
def _memoized(value=1):
    return value


@functools.singledispatch
def _dispatched(value=1):
    return value


def _last_param_default(value, flag=True):
    return value, flag


def _annotated(value: int = 1) -> int:
    return value


_connect("a", 5)
_connect("b", timeout=7)
_retry(4, delay=2)
_fetch("a", 3)
_pair(1, 2)
_pair(second=3, first=4)
_spread(*[1])
_mapping(**{"value": 2})
_spread(1)
_mapping(1)
_memoized(1)
_dispatched(1)
_last_param_default("x", False)
_last_param_default("y", flag=True)
list(map(_passed_as_value, [1]))
_passed_as_value(1)
_annotated(2)


class _Client:
    def send_frames(self, data, flush=False):
        return data, flush

    def close_stream(self, force=False):
        return force

    def __init__(self, mode=0):
        self.mode = mode

    def run(self):
        self.send_frames("x", True)
        self.send_frames("y", flush=False)
        return self.close_stream()


class _Holder:
    pass


class _Derived(_Holder):
    def ping_host(self, quick=True):
        return quick

    def again(self):
        return self.ping_host()


def _use_client(client):
    return client.close_stream(True)


_Derived(0).ping_host(False)


class _Relay:
    def hop(self, value=1):
        return value

    def pass_on(self, other):
        return other.hop() + self.hop(2)

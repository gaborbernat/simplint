def connect(host, timeout=30):
    return host, timeout


class Client:
    def send(self, data, flush=False):
        return data, flush


def _sync(value=1):
    return value


connect("a", 5)
Client().send("b", True)
_sync(2)

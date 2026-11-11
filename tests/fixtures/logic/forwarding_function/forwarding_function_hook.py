from pathlib import Path


class Store:
    def load(self, name):
        return self._make_path(name)

    def _make_path(self, name):
        return Path(name)


class RemoteStore(Store):
    def _make_path(self, name):
        return Path("remote") / name


class Lookup:
    def __init__(self, client):
        self.client = client

    def fetch(self, name):
        return self._get(name)

    def _get(self, name):
        return self.client.get(name)


class Memoized:
    def __init__(self, client):
        self.client = client

    def fetch(self, name):
        return self._get(name)

    def _get(self, name):
        return self.client.get(name)

    def getter(self):
        return self._get

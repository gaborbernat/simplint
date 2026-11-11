class Cache:
    def __init__(self, client):
        self.client = client

    def fetch(self, name):
        return self._get(name), self._own(name)

    def _get(self, name):
        return self.client.get(name)

    def _own(self, name):
        return self(name)

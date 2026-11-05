from unittest import TestCase, mock
from unittest.mock import patch


class ConfigTest(TestCase):
    def test_home(self):
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(patcher.stop)
        home.return_value = "/home"

    def test_object(self):
        patcher = mock.patch.object(Config, "debug", True)
        debug = patcher.start()
        self.addCleanup(patcher.stop)
        debug.return_value = False

    def test_environment(self):
        patcher = patch.dict("os.environ", {"HOME": "/home"})
        environment = patcher.start()
        self.addCleanup(patcher.stop)
        environment.clear()

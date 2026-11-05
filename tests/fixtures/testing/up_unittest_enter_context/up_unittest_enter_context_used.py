from unittest import TestCase, mock


class ConfigTest(TestCase):
    def test_home(self):
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(patcher.stop)
        home.return_value = "/home"
        patcher.stop()

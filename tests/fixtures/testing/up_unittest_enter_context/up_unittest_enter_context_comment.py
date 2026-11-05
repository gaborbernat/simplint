from unittest import TestCase, mock


class ConfigTest(TestCase):
    def test_home(self):
        patcher = mock.patch("os.getcwd")
        # The cleanup stops the patch.
        home = patcher.start()
        self.addCleanup(patcher.stop)
        home.return_value = "/home"

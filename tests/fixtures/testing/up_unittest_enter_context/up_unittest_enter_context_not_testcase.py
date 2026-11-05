from unittest import TestCase, mock


class ConfigTest(Base):
    def test_home(self):
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(patcher.stop)
        home.return_value = "/home"


class OtherTest(TestCase):
    def test_other(self):
        patcher = other.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(patcher.stop)
        value = 1
        other_value = value
        self.addCleanup(value)
        patcher = mock.patch("os.getcwd")
        home = patcher.start(1)
        self.addCleanup(patcher.stop)
        patcher = mock.patch("os.getcwd")
        home = other.start()
        self.addCleanup(patcher.stop)
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(other)


class ShapeTest(TestCase):
    def test_shapes(self):
        self.enable()
        home = patcher.start()
        self.addCleanup(patcher.stop)
        patcher = mock.patch("os.getcwd")
        home = patcher
        self.addCleanup(patcher.stop)
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        cleanup = 1
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        home
        patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup()
        self.patcher = mock.patch("os.getcwd")
        home = patcher.start()
        self.addCleanup(patcher.stop)

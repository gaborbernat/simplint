from unittest import TestCase


class ConfigTest(TestCase):
    def test_name(self):
        self.assertTrue(name.startswith("demo"))
        self.assertTrue(name.endswith("demo"), "suffix")
        self.assertFalse(name.startswith(prefix), msg="prefix")
        self.assertFalse(name.endswith("demo"))
        self.assertTrue(hasattr(config, "debug"))
        self.assertFalse(hasattr(config, "debug"), msg="debug")
        self.assertTrue(issubclass(Child, Base))
        self.assertFalse(issubclass(Child, (Base, Other)))

    def test_not_helpers(self):
        self.assertTrue(name.startswith("demo", 1))
        self.assertTrue(name.lower())
        self.assertTrue(len(name))
        self.assertTrue(name.startswith("demo"), "a", "b")
        self.assertTrue(name.startswith("demo"), msg="message", extra=1)
        self.assertTrue(name.startswith(*args))
        self.assertTrue(name.startswith("demo"), *args)
        self.assertEqual(name.startswith("demo"), True)
        self.assertTrue(name.startswith("demo"), other)
        check(name.startswith("demo"))
        self.assertTrue(f()())
        self.assertTrue(hasattr(config, "debug", 1))
        self.assertTrue(hasattr(*args))
        self.assertTrue(name.upper("x"))
        self.assertTrue(isinstance(name, str))
        self.assertTrue(flag)


class Helper:
    def test_name(self):
        self.assertTrue(name.startswith("demo"))

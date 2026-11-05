from unittest import TestCase


class ConfigTest(TestCase):
    def test_name(self):
        self.assertTrue(
            # The prefix comes from the fixture.
            name.startswith("demo")
        )

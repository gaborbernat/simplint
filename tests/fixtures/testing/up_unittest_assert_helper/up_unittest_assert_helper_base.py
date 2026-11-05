from third_party import Base


class ConfigTest(Base):
    def test_name(self):
        self.assertTrue(name.startswith("demo"))

SETTINGS = {"retries": 3}


class Settings:
    def apply(self, address: str) -> str:
        return address


def make_settings() -> Settings:
    return Settings()

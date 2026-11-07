class Execute:
    def __init__(self, colored: bool) -> None:
        self.colored = colored


class Project:
    """A project created for a test.

    :param files: the files of the project
    :param path: the directory of the project
    """

    def __init__(self, files: dict[str, str], path: str) -> None:
        self.files = files
        self.path = path
        self.counter = 0

    def patch_execute(self, colored: bool) -> None:
        """Patch the execution.

        :param colored: whether to color the output
        """

        class MockExecute(Execute):
            def __init__(self, colored: bool, exit_code: int) -> None:
                self.exit_code = exit_code
                super().__init__(colored)

        class MockExecuteInstance(Execute):
            def __init__(self, exit_code: int) -> None:
                self.exit_code = exit_code

        self.patched = MockExecute(colored, 0)

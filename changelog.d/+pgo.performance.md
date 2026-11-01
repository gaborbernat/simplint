Release wheels for x86_64 Linux (glibc and musl), x64 Windows and arm64 macOS are built with profile-guided
optimization, trained on `simplint check` and `simplint check --fix` over pinned sdists of black, pip and botocore. On
`sympy` and `django` this runs about 6% fewer instructions in check mode and about 12% fewer with `--fix`.

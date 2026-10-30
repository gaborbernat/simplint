simplint
========

``simplint`` is a companion linter for :gh:`astral-sh/ruff`. It enforces the parts of the ``simp`` house style that
Ruff has no rule for, such as definition order, ``Final`` constants, explicit exports and test layout. It is built on
Ruff's own parser crates, so the 264 files of :gh:`tox-dev/tox` lint in about 10 ms, and it fixes what it reports
wherever the fix is mechanical. See the `release history <https://github.com/gaborbernat/simplint/releases>`_ for
version changes.

.. code-block:: console

    $ simplint check --select SMP050,SMP101 src/tox/util/path.py src/tox/util/spinner.py
    src/tox/util/path.py:16:1: SMP050 `_CACHEDIR_TAG` is used by one function only; define it there
    src/tox/util/path.py:16:1: SMP101 [*] `_CACHEDIR_TAG` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:41:1: SMP101 [*] `MISS_DURATION` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:181:1: SMP050 `_PERIODS` is used by one function only; define it there
    src/tox/util/spinner.py:181:1: SMP101 `_PERIODS` is never reassigned; annotate it `Final`
    Found 5 issues.
    [*] 2 fixable with `--fix`.
    2 hidden fixes can be enabled with `--unsafe-fixes`.

Install
-------

simplint ships as a single binary inside a Python wheel, like Ruff, so it needs no Python at runtime. An isolated tool
environment keeps it apart from the project it lints.

.. tab:: uv

    .. code-block:: bash

        uv tool install simplint
        simplint --help

.. tab:: pipx

    .. code-block:: bash

        python -m pip install pipx-in-pipx --user
        pipx install simplint
        simplint --help

.. tab:: pip

    .. code-block:: bash

        python -m pip install --user simplint
        simplint --help

    A user installation shares the interpreter's package set. Prefer ``uv`` or ``pipx`` when an operating system or
    package manager owns that interpreter.

Pre-commit
----------

Add the hook to ``.pre-commit-config.yaml`` and set ``rev`` to the required release. The hook lints the whole project
on every run, since several rules need to know which names other modules import:

.. code-block:: yaml

    - repo: https://github.com/gaborbernat/simplint
      rev: ""
      hooks:
        - id: simplint

See :gh:`pre-commit/pre-commit` for installation and update commands.

Running it
----------

``simplint check [PATHS]`` lints the Python files under each path, honoring ``.gitignore``. Add ``--fix`` to apply safe
fixes, ``--unsafe-fixes`` to also apply fixes that can change behavior, and ``--diff`` to print the fixes without
writing them. ``--select`` and ``--ignore`` take comma-separated rule codes, code prefixes or rule names.
``simplint rules`` lists every rule and whether it fixes, and ``simplint rule SMP111`` prints one rule's documentation.

Fixes rewrite code without formatting it, so run the formatter afterwards, the same way you would after
``ruff check --fix``:

.. code-block:: bash

    simplint check --fix . && ruff check --fix . && ruff format .

.. toctree::
   :hidden:

   self
   configuration
   rules
   changelog

See the `configuration reference <https://simplint.readthedocs.io/en/latest/configuration.html>`_ for settings and the
`rules reference <https://simplint.readthedocs.io/en/latest/rules.html>`_ for every rule with examples and fix safety.

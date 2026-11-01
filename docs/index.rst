simplint
========

.. raw:: html

    <section class="hero">
      <img class="hero-logo" src="_static/logo.png" alt="">
      <p class="hero-title">Python that is <span>easier to read and review</span>.</p>
      <p class="hero-tagline">A fast companion linter for Ruff, written in Rust. It checks for what reviewers keep
      pointing out, and fixes most of what it finds.</p>
      <div class="hero-actions">
        <a class="primary" href="#install">Get started</a>
        <a class="secondary" href="rules.html">Browse the rules</a>
        <a class="secondary" href="https://github.com/gaborbernat/simplint">GitHub</a>
      </div>
    </section>

``simplint`` is a companion linter for `Ruff <https://docs.astral.sh/ruff/>`__. It adds checks that make Python code
easier to read and review: `definitions ordered like a book
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP020.html>`__, `explicit exports
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP103.html>`__, `Final constants
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP101.html>`__, `flat test modules
<https://simplint.readthedocs.io/en/latest/_generated/rules/SMP200.html>`__, and more. The rules are opinionated. They
reflect the maintainer's preferences while aiming to be as objective as possible, so select the ones that fit your
project.

simplint is written in `Rust <https://rust-lang.org/>`__ on top of Ruff's parser crates, and lints the 264 files of
:gh:`tox-dev/tox` in about 10 ms. When the fix for a report is mechanical, it can apply it for you. The
`release history <https://github.com/gaborbernat/simplint/releases>`__ lists the changes in each version.

.. grid:: 1 1 2 2
    :gutter: 3

    .. grid-item-card:: ⚡ Fast
        :class-card: feature

        Rust on Ruff's parser: a whole project lints in milliseconds, fast enough for a pre-commit hook.

    .. grid-item-card:: 🛠️ Fixes, not just reports
        :class-card: feature

        Most rules rewrite the code for you. Fixes that could change behavior run only with ``--unsafe-fixes``.

    .. grid-item-card:: 🔗 Knows the whole project
        :class-card: feature

        An import index across every file keeps fixes from renaming or moving a name another module uses.

    .. grid-item-card:: 🤝 Built to pair with Ruff
        :class-card: feature

        No overlap with Ruff's rules, the same ``noqa`` comments, and the same ``pyproject.toml``.

A run over two tox modules, limited to two rules:

.. code-block:: console

    $ simplint check --output-format concise --select SMP050,SMP101 src/tox/util/path.py src/tox/util/spinner.py
    src/tox/util/path.py:16:1: SMP050 `_CACHEDIR_TAG` is used by one function only; define it there
    src/tox/util/path.py:16:1: SMP101 `_CACHEDIR_TAG` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:41:1: SMP101 `MISS_DURATION` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:181:1: SMP050 `_PERIODS` is used by one function only; define it there
    src/tox/util/spinner.py:181:1: SMP101 `_PERIODS` is never reassigned; annotate it `Final`
    Found 5 errors.
    4 unsafe fixes available with `--fix --unsafe-fixes`.

Install
-------

simplint ships as a single binary inside a Python wheel, like Ruff, so it needs no Python at runtime. Install it as a
tool with `uv <https://docs.astral.sh/uv/concepts/tools/>`__ or `pipx <https://pipx.pypa.io/stable/>`__, which keeps it
apart from the project it lints.

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

    A `user installation <https://pip.pypa.io/en/stable/user_guide/#user-installs>`__ shares the interpreter's package
    set. Prefer ``uv`` or ``pipx`` when an operating system or package manager owns that interpreter.

Quick start
-----------

Run simplint from the project root. Several rules need to know which names other modules import, so check the whole
project, tests included:

.. code-block:: bash

    simplint check .

``simplint check`` works like `ruff check <https://docs.astral.sh/ruff/linter/>`__: it finds the same files, honors
the same ``exclude`` settings and `.gitignore <https://git-scm.com/docs/gitignore>`__, prints in the same formats and
exits the same way. It lints every ``.py`` file under the paths you give it, the current directory by default, and
``-`` reads one file from standard input. By default each report shows the offending code, as ``ruff check`` does;
``--output-format concise`` prints one line per report, and ``[*]`` marks a report with a safe fix. The command exits
with status 1 while any report remains, and with status 2 when it cannot run, such as on invalid configuration.

To apply the safe fixes, add ``--fix``. Run simplint between Ruff's `linter <https://docs.astral.sh/ruff/linter/>`__
and its `formatter <https://docs.astral.sh/ruff/formatter/>`__:

.. code-block:: bash

    ruff check --fix .
    simplint check --fix .
    ruff format .

One round is final: running the three again changes nothing. simplint leaves layout to ``ruff format``, so its output
need not be formatted, only something one ``ruff format`` run finishes. When a fix gives one of the project's Ruff rules
something to report that the file was clean for, simplint applies Ruff's own fix for it, as for an import left unused;
when Ruff has no fix, simplint withdraws its own fix and keeps the report.

The options of ``simplint check`` are the ones ``ruff check`` has, with the same meaning:

- ``--unsafe-fixes`` also applies the fixes that can change behavior; ``--diff`` prints the fixes as a unified diff
  and writes nothing; ``--fix-only`` fixes without reporting what is left; ``--show-fixes`` lists what was fixed.
- ``--select``, ``--extend-select`` and ``--ignore`` take comma-separated rule codes (``SMP111``), code prefixes
  (``SMP2``) or rule names (``dataclass-options``). ``--select`` replaces the selection from ``pyproject.toml``.
  ``--fixable``, ``--unfixable`` and ``--extend-fixable`` pick which of the selected rules may fix, and
  ``--per-file-ignores`` and ``--extend-per-file-ignores`` take ``PATTERN:CODE`` pairs.
- ``--output-format`` picks one of Ruff's formats: ``full``, ``concise``, ``grouped``, ``json``, ``json-lines``,
  ``junit``, ``github``, ``gitlab``, ``pylint``, ``rdjson``, ``azure`` or ``sarif``. ``--statistics`` counts the
  reports per rule.
- ``--watch`` checks again whenever a Python or TOML file changes. ``--add-noqa`` writes a ``# noqa`` comment for
  every report, and ``--add-ignore`` a ``# simplint: ignore[...]`` comment; both take an optional reason, as in
  ``--add-noqa="legacy code"``. ``--show-files`` lists the files a check would cover and ``--show-settings`` prints
  the settings for the first of them.
- ``--config`` takes a configuration file or a ``KEY = VALUE`` override. A ``[tool.simplint]`` key, such as
  ``--config "select = ['SMP1']"``, overrides simplint's settings, and any other key Ruff's, such as
  ``--config "target-version = 'py312'"``.
- ``--exit-zero``, ``--exit-non-zero-on-fix``, ``--ignore-noqa``, ``--isolated``, ``--color``, ``--target-version``,
  ``--exclude``, ``--extend-exclude``, ``--no-cache``, ``--cache-dir`` and the ``--quiet``, ``--silent`` and
  ``--verbose`` log levels behave as in Ruff.
- ``--preview`` turns on Ruff's preview mode. simplint has no preview rules, so it only changes the output, which names
  rules instead of showing their codes, and lets simplint's fixes account for Ruff's preview rules.

See :doc:`cli` for every option.

``simplint rules`` lists every rule and whether it has a fix. As in Ruff, ``simplint rule SMP111`` prints the
documentation of one rule and ``simplint rule --all`` of every rule, ``simplint config`` describes the
``[tool.simplint]`` options, ``simplint linter`` lists the rule groups, ``simplint clean`` removes its caches from the
``.ruff_cache`` directories under the current directory and ``simplint version`` prints the version. All but
``clean`` take ``--output-format json``.

Pre-commit
----------

Add the hook to the ``.pre-commit-config.yaml`` of `pre-commit <https://pre-commit.com/>`__ and set ``rev`` to a
`release tag <https://github.com/gaborbernat/simplint/releases>`__:

.. code-block:: yaml

    - repo: https://github.com/gaborbernat/simplint
      rev: ""
      hooks:
        - id: simplint

The hook runs ``simplint check --fix`` on the whole project rather than on the changed files, because several rules need
to know which names other modules import. List it after Ruff's ``ruff-check`` hook and before its ``ruff-format`` hook,
the order above. It is a `Rust hook <https://pre-commit.com/#rust>`__: pre-commit builds simplint from source with
Cargo, so the first run takes longer than later ones.

.. toctree::
   :hidden:

   Home <self>
   configuration
   rules
   changelog

The `configuration reference <https://simplint.readthedocs.io/en/latest/configuration.html>`__ covers settings and
suppression comments. The `rules reference <https://simplint.readthedocs.io/en/latest/rules.html>`__ documents every
rule with examples and fix safety.

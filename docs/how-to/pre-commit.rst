Run simplint with pre-commit
############################

Add the hook from the `simplint-pre-commit <https://github.com/gaborbernat/simplint-pre-commit>`__ mirror to the
``.pre-commit-config.yaml`` of `pre-commit <https://pre-commit.com/>`__, and set ``rev`` to one of
`its tags <https://github.com/gaborbernat/simplint-pre-commit/tags>`__:

.. code-block:: yaml

    - repo: https://github.com/gaborbernat/simplint-pre-commit
      rev: ""
      hooks:
        - id: simplint

What to know about the hook:

- It runs ``simplint check --fix`` on the **whole project** rather than on the changed files, because several rules
  need to know which names other modules import.
- List it after `Ruff <https://docs.astral.sh/ruff/>`__'s ``ruff-check`` hook and before its ``ruff-format`` hook, the
  order shown in the :doc:`tutorial </tutorial/getting-started>`.
- The mirror installs the wheel from `PyPI <https://pypi.org/project/simplint>`__, so it needs no Rust toolchain and
  runs on `pre-commit.ci <https://pre-commit.ci>`__.
- The mirror gets a tag for each release minutes after it reaches PyPI.

To run a commit that is not released yet, use ``https://github.com/gaborbernat/simplint`` as the ``repo`` and the
commit as the ``rev``.

.. warning::

    That is a `Rust hook <https://pre-commit.com/#rust>`__: pre-commit builds simplint from source with the Rust
    version the repository pins. The toolchain must be installed, which it is not on pre-commit.ci.

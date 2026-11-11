Suppress a report
#################

simplint honors the same `noqa comments <https://docs.astral.sh/ruff/linter/#error-suppression>`__ as `Ruff
<https://docs.astral.sh/ruff/>`__. Put the comment on the line where the report starts, and give a reason after the
codes:

.. code-block:: python

    x = solve(matrix)  # noqa: SMP101 matches the paper's notation

- A report on a multi-line string or a line continued with ``\`` takes the comment on the last line of the string or
  continuation, as in Ruff.
- A bare ``# noqa`` suppresses every simplint rule on its line, but
  :doc:`suppression-without-reason </rules/SMP003>` reports it for giving no reason.
- An invalid directive, such as ``# noqa: 401``, makes a fix run warn once, not once per rule it would suppress. Both
  ``--fix`` and ``--diff`` print that warning:

  .. code-block:: console

      $ simplint check --fix --output-format concise --select SMP001 a.py
      warning: Invalid `# noqa` directive on a.py:6: expected a comma-separated list of codes (e.g., `# noqa: F401, F841`).
      Found 2 errors (2 fixed, 0 remaining).

The suppression comments at a glance:

.. list-table::
    :header-rows: 1
    :widths: 40 60

    * - Comment
      - Suppresses
    * - ``# noqa: SMP101``
      - The listed rules on its line.
    * - ``# simplint: ignore[SMP101]``
      - The listed rules on its line, the way Ruff's ``# ruff: ignore[...]`` does for Ruff's rules.
    * - ``# ruff: noqa: SMP101`` (or ``# flake8: noqa: SMP101``) on its own line
      - The listed rules in the whole file.
    * - ``# ruff: noqa`` on its own line
      - Every rule but :doc:`suppression-without-reason </rules/SMP003>` in the whole file.
    * - ``# ruff: file-ignore[SMP101]`` at module scope
      - The listed rules in the whole file.

The file-level comments work as in Ruff.

To silence every remaining report at once:

- ``simplint check --add-noqa`` writes a ``# noqa`` comment for every report left, merged into the line's existing
  ``# noqa`` comment. It skips a line whose ``# noqa`` is blanket, since a blanket ``# noqa`` takes no codes.
- ``--add-ignore`` writes ``# simplint: ignore`` comments the same way, on a line with a blanket ``# noqa`` too, where
  the comment silences the reports the blanket one leaves.
- Both take a reason to write after the codes: ``--add-noqa="matches the paper's notation"``.
- Neither takes standard input: ``simplint check --add-noqa -`` fails with an error.

A reason replaces the one the comment already has, issue numbers such as ``#12`` included, and a comment after that
reason stays. With ``--add-noqa=why`` on a line that reports SMP101:

.. code-block:: python

    x = 1  # noqa: E501 see issue #12 for why  # type: ignore[misc]

becomes

.. code-block:: python

    x = 1  # noqa: E501, SMP101 why  # type: ignore[misc]

The :doc:`unused-ignore </rules/SMP009>` rule reports obsolete selectors in ``# simplint: ignore[...]``
comments. Run ``simplint check --extend-select SMP009 --fix`` to remove them. The fix re-lints uncertain removals so a
comment that affects another rule stays in place. It leaves ``# noqa`` and configuration-level ignores alone.

Run with the project's normal rule selection. A narrow ``--select`` can make ignores for other rules appear unused.

.. important::

    Ruff does not know the ``SMP`` codes. List them in
    `lint.external <https://docs.astral.sh/ruff/settings/#lint_external>`__ so that Ruff keeps these comments instead
    of reporting them as unused (`unused-noqa <https://docs.astral.sh/ruff/rules/unused-noqa/>`__):

    .. code-block:: toml

        [tool.ruff]
        lint.external = ["SMP"]

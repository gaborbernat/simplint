Suppress a report
#################

simplint honors the same `noqa comments <https://docs.astral.sh/ruff/linter/#error-suppression>`__ as Ruff. Put the
comment on the line where the report starts, and give a reason after the codes:

.. code-block:: python

    x = solve(matrix)  # noqa: SMP101 matches the paper's notation

- A report on a multi-line string or a line continued with ``\`` takes the comment on the last line of the string or
  continuation, as in Ruff.
- A bare ``# noqa`` suppresses every simplint rule on its line, but
  :doc:`suppression-without-reason </_generated/rules/SMP003>` reports it for giving no reason.

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
      - Every rule but :doc:`suppression-without-reason </_generated/rules/SMP003>` in the whole file.
    * - ``# ruff: file-ignore[SMP101]`` at module scope
      - The listed rules in the whole file.

The file-level comments work as in Ruff.

To silence every remaining report at once:

- ``simplint check --add-noqa`` writes a ``# noqa`` comment for every report left, merged into the line's existing
  ``# noqa`` comment.
- ``--add-ignore`` writes ``# simplint: ignore`` comments the same way.
- Both take a reason to write after the codes: ``--add-noqa="matches the paper's notation"``.

.. important::

    Ruff does not know the ``SMP`` codes. List them in
    `lint.external <https://docs.astral.sh/ruff/settings/#lint_external>`__ so that Ruff keeps these comments instead
    of reporting them as unused (`unused-noqa <https://docs.astral.sh/ruff/rules/unused-noqa/>`__):

    .. code-block:: toml

        [tool.ruff]
        lint.external = ["SMP"]

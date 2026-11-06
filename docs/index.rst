simplint
========

.. raw:: html

    <section class="hero">
      <img class="hero-logo" src="_static/logo.webp" alt="" width="256" height="256">
      <p class="hero-title">Python that is <span>easier to read and review</span>.</p>
      <p class="hero-tagline">A fast companion linter for Ruff, written in Rust. It checks for what reviewers keep
      pointing out, and fixes most of what it finds.</p>
      <div class="hero-actions">
        <a class="primary" href="tutorial/getting-started.html">Get started</a>
        <a class="secondary" href="rules.html">Browse the rules</a>
        <a class="secondary" href="https://github.com/gaborbernat/simplint">GitHub</a>
      </div>
    </section>

``simplint`` is a companion linter for `Ruff <https://docs.astral.sh/ruff/>`__. It adds checks that make Python code
easier to read and review:

- :doc:`definitions ordered like a book <_generated/rules/SMP201>`
- :doc:`explicit exports <_generated/rules/SMP403>`
- :doc:`Final constants <_generated/rules/SMP601>`
- :doc:`flat test modules <_generated/rules/SMP801>`
- and more

The rules are opinionated. They reflect the maintainer's preferences while aiming to be as objective as possible, so
select the ones that fit your project.

simplint is written in `Rust <https://rust-lang.org/>`__ on top of Ruff's parser crates, and lints the 264 files of
:gh:`tox-dev/tox` in about 10 ms. When the fix for a report is mechanical, it can apply it for you. The
:doc:`release history <changelog>` lists the changes in each version.

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

        Few rules overlap with Ruff's, and both tools read the same ``noqa`` comments and ``pyproject.toml``.

A run over two tox modules, limited to two rules:

.. code-block:: console

    $ simplint check --output-format concise --select SMP202,SMP601 src/tox/util/path.py src/tox/util/spinner.py
    src/tox/util/path.py:16:1: SMP202 `_CACHEDIR_TAG` is used by one function only; define it there
    src/tox/util/path.py:16:1: SMP601 `_CACHEDIR_TAG` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:41:1: SMP601 `MISS_DURATION` is never reassigned; annotate it `Final`
    src/tox/util/spinner.py:181:1: SMP202 `_PERIODS` is used by one function only; define it there
    src/tox/util/spinner.py:181:1: SMP601 `_PERIODS` is never reassigned; annotate it `Final`
    Found 5 errors.
    4 unsafe fixes available with `--fix --unsafe-fixes`.

Documentation
-------------

.. grid:: 1 2 2 4
    :gutter: 3

    .. grid-item-card:: Tutorial
        :link: tutorial/getting-started
        :link-type: doc

        For when you are new to simplint and want to run it on a project, step by step.

    .. grid-item-card:: How-to guides
        :link: how-to/index
        :link-type: doc

        For when you want to get one task done, such as choosing which rules run.

    .. grid-item-card:: Reference
        :link: rules
        :link-type: doc

        For when you need the exact name, option or default of something.

    .. grid-item-card:: Explanation
        :link: explanation/how-it-works
        :link-type: doc

        For when you want to understand how simplint works and how it fits with Ruff.

.. toctree::
   :hidden:

   Home <self>
   Getting started <tutorial/getting-started>
   how-to/index

.. toctree::
   :hidden:
   :caption: Reference

   rules
   configuration
   cli
   changelog

.. toctree::
   :hidden:
   :caption: Explanation

   explanation/how-it-works
   explanation/simplint-and-ruff

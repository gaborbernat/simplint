simplint
========

.. raw:: html

    <section class="hero">
      <img class="hero-logo" src="_static/logo.webp" alt="" width="256" height="256">
      <p class="hero-title">Python that is <span>easier to read and review</span>.</p>
      <p class="hero-tagline">A companion linter for <a href="https://docs.astral.sh/ruff/">Ruff</a> that checks
      Python definitions, imports, exports and tests.</p>
      <div class="hero-actions">
        <a class="primary" href="tutorial/getting-started.html">Get started</a>
        <a class="secondary" href="rules.html">Browse the rules</a>
        <a class="secondary" href="https://github.com/gaborbernat/simplint">GitHub</a>
      </div>
    </section>

.. grid:: 1 1 2 2
    :gutter: 3

    .. grid-item-card:: Rules
        :class-card: feature

        Order :doc:`definitions after their callers <rules/SMP201>`, declare :doc:`module exports <rules/SMP403>`,
        annotate :doc:`constants with Final <rules/SMP601>` or :doc:`flatten test classes <rules/SMP801>`.
        Select the rules that fit your project.

    .. grid-item-card:: Fixes
        :class-card: feature

        Most rules offer a fix. Apply safe fixes with ``simplint check --fix``. Add ``--unsafe-fixes`` to include
        changes that may affect behavior.

    .. grid-item-card:: Cross-file checks
        :class-card: feature

        Check the whole project, including tests, so rules can see where another module imports a name.

    .. grid-item-card:: Ruff configuration
        :class-card: feature

        simplint uses Ruff's file exclusions and reads its rule settings to check for conflicting fixes.
        Both tools recognize ``# noqa`` comments.

For example, given a file named ``module.py`` containing ``x = 1``:

.. code-block:: console

    $ simplint check --select SMP101 --output-format concise module.py
    module.py:1:1: SMP101 Single-letter name `x`; use a descriptive name
    Found 1 error.

Documentation
-------------

.. grid:: 1 2 2 4
    :gutter: 3

    .. grid-item-card:: Tutorial
        :link: tutorial/getting-started
        :link-type: doc

        Install simplint and run your first check.

    .. grid-item-card:: How-to guides
        :link: how-to/index
        :link-type: doc

        Choose rules, configure a hook or suppress a report.

    .. grid-item-card:: Reference
        :link: rules
        :link-type: doc

        Look up a rule, setting or command-line option.

    .. grid-item-card:: Explanation
        :link: explanation/how-it-works
        :link-type: doc

        Read how cross-file checks work and how simplint handles Ruff findings.

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

   explanation/design-principles
   explanation/how-it-works
   explanation/simplint-and-ruff

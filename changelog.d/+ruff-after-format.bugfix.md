Fixes no longer leave Ruff work that only shows up after `ruff format`: a nested `if` or `with` that a fix lets fit on
one line gets merged, an import block a fix joins across a `;` gets sorted, and a `# noqa: E501` on a line a fix changes
stays.

# Threat model

The [security policy](SECURITY.md) sets the reporting process and takes precedence if the two documents disagree.

## What a lint run can access

The caller chooses the command line, working directory and environment. A contributor to a pull request can control the
repository's source files, configuration, project metadata, documentation and symlinks.

| Input or operation                 | How simplint uses it                                                                                                                                                                                                                                                                                                                                  |
| ---------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Python source and stubs            | Ruff's parser reads source as data. simplint lints Python files and indexes names declared by stubs. It does not import or execute project code or make network requests while linting.                                                                                                                                                               |
| Project metadata and documentation | The index reads files such as `pyproject.toml`, `.pth` files, Sphinx configuration and documentation pages to find imports, entry points and public names. These can affect findings even when the command names a single Python file.                                                                                                                |
| Ruff and simplint configuration    | Settings control file discovery, rule selection and fixes. Ruff can read a configuration file referenced by `extend`. If that file is invalid and lies outside the project root, the error identifies it by path without printing its contents.                                                                                                       |
| Source edits                       | `--fix`, `--add-noqa` and `--add-ignore` can edit a linted file. simplint resolves the target and checks that it lies under a path the caller chose. `--diff` prints proposed edits.                                                                                                                                                                  |
| Cache and reports                  | A repository-configured cache directory must resolve inside the project root. If it points outside, simplint tries Ruff's default cache inside the root. It disables caching if that location also leads outside. The caller can choose a cache elsewhere with `--cache-dir` or `RUFF_CACHE_DIR`, or send diagnostics to a file with `--output-file`. |

## Threats and controls

- **Unexpected source edits.** The edit goes back to the file simplint read. A symlink named on the command line
  includes its target in the run. A file symlink discovered during a directory walk is skipped with a warning if its
  target lies outside all the linted paths. Before writing, simplint checks that the file still contains the source it
  read.
- **Cache cleanup.** Automatic cleanup removes stale build directories only when their names match simplint's build
  format and their entries are regular cache files. It skips symlinks to directories.
- **Broken fixes.** The fixer reparses the edited source. A fix that produces invalid syntax leaves the original file
  intact. A safe fix that produces valid Python but changes behavior is a correctness bug.
- **Denial of service.** The fixer limits iterations within a file and rounds across files. Workers use a 64 MiB stack
  for nested input. A small input that hangs simplint or exhausts memory is a vulnerability, as is a crash on malformed
  input.

A lint result says which rules a file violated. A file that passes can still execute malicious code if someone runs it.

## Release chain

The release App pushes the release commit and tag through the shared
[tox-dev workflows](https://github.com/tox-dev/github-workflows). The build gets its maturin version from `pylock.toml`.
The release-tools update workflow uses a seven-day upload cutoff by default. The publishing job uses
[PyPI trusted publishing](https://docs.pypi.org/trusted-publishers/) with attestations. GitHub publishes an immutable
release. Workflow files pin reusable external workflows to commits, and `actions.lock` records the commits for actions
referenced by version.

## Related policies

The [Node.js threat model](https://github.com/nodejs/node/blob/main/SECURITY.md#the-nodejs-threat-model) identifies the
inputs it trusts.
[DOMPurify](https://github.com/cure53/DOMPurify/blob/main/SECURITY.md#what-we-consider-a-vulnerability) classifies
findings by what the attacker controls. Other Python linters document how they handle reports.
[Ruff](https://github.com/astral-sh/.github/blob/main/SECURITY.md) accepts reports by email and discloses critical
vulnerabilities through GitHub advisories. [Flake8](https://github.com/PyCQA/flake8/blob/main/.github/SECURITY.md) and
[Pylint](https://github.com/pylint-dev/pylint/blob/main/.github/SECURITY.md) direct reporters to Tidelift.

Review this model whenever simplint gains a new input source, network access, a way to run code or a place to write
files, and whenever the project publishes an advisory.

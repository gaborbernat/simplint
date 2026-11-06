Flags modules and packages named for a grab bag of code, such as `utils`, `helpers` or `common`. The names come from a
list you can change.

## Why is this bad?

A module named `utils` collects whatever did not fit elsewhere, and its name says nothing about any of it. Imports from
it read the same whatever they bring in, and the module keeps growing.

## Example

```python report file=utils.py
def slugify(text: str) -> str:
    return text.lower().replace(" ", "-")
```

Use instead:

```python ok file=slugs.py
def slugify(text: str) -> str:
    return text.lower().replace(" ", "-")
```

## Configuration

- [`naming.catch-all-modules`](https://simplint.readthedocs.io/en/latest/configuration/rule-settings.html#naming) in
  `[tool.simplint]`: the module and package names to report. A list replaces the default one, which holds `utils`,
  `util`, `utilities`, `helpers`, `helper`, `common` and `misc`.

  ```toml config
  [tool.simplint.naming]
  catch-all-modules = ["utils", "helpers", "misc"]
  ```

  ```python report file=helpers.py
  def parse() -> None:
      pass
  ```

## What it does

### Active on

- **Modules**: a file named for an entry of the list, such as `utils.py`.

  ```python report file=common.py
  VALUE = 1
  ```

- **Packages**: an `__init__.py` in a folder named for an entry of the list, such as `utils/__init__.py`.

  ```python report file=utils/__init__.py
  VALUE = 1
  ```

### Exceptions

- **Modules in a catch-all package**: `utils/slugs.py` keeps its own name, and the package is reported through its
  `__init__.py`. A folder without an `__init__.py` is not reported.

  ```python ok file=utils/slugs.py
  def slugify(text: str) -> str:
      return text.lower().replace(" ", "-")
  ```

- **Names outside the list**: `models.py` is the name Django reads for a project's models, and `types.py` or
  `interfaces.py` usually holds one kind of code, type aliases or protocols, so the default list leaves them out. A
  project whose `types` is a grab bag adds it to `catch-all-modules`.

  ```python ok file=models.py
  VALUE = 1
  ```

  ```python ok file=types.py
  Slug = str
  ```

## Fix

No fix is offered. A rename changes every import of the module, and a split needs to know what each part holds.

## Other tools

- [wemake-python-styleguide](https://github.com/wemake-services/wemake-python-styleguide) reports modules named for a
  blacklisted word, such as `utils`, as
  [WPS100](https://wemake-python-styleguide.readthedocs.io/en/latest/pages/usage/violations/naming.html#wemake_python_styleguide.violations.naming.WrongModuleNameViolation).

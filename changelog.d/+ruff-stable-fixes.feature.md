Fixes now leave nothing for Ruff to redo when `simplint check --fix` runs between `ruff check --fix` and `ruff format`:
a Ruff violation a fix introduces, or a Ruff fix it makes possible such as a return type Ruff can now infer, gets Ruff's
own fix in the same run, and otherwise the fix is withdrawn and the report kept. Layout is left to `ruff format`, save a
line no formatting can bring under the line length, such as a name or string too long by itself.

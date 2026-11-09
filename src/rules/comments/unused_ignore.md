Reports selectors in `# simplint: ignore[...]` comments that no longer suppress a report under the active rule settings.
Enable it with `--extend-select SMP009` when checking for obsolete ignores.

## Why is this bad?

An obsolete ignore makes it harder to tell which exceptions still matter. It can also hide a new report if the same rule
starts applying to that line later.

## Example

```python report
value = 1  # simplint: ignore[single-letter-name]
```

Use instead:

```python ok
value = 1
```

## What it does

The rule tests removals one selector at a time, then offers one safe fix for the file. It re-lints each proposed edit
that might be unused and keeps the selector if removing it exposes any report, including one from a different rule. When
a comment has several selectors, the fix keeps those that still suppress reports. A whole comment is removed when its
last selector is unused.

The rule checks the rules currently selected for the file. If you run a narrow `--select`, selectors for rules outside
that selection can appear unused. Use the project's normal settings when cleaning up ignores.

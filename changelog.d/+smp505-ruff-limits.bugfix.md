`single-caller-helper` avoids suggesting inlining when it would cross enabled Ruff limits for nested blocks or try
statements, or trigger [raise-within-try](https://docs.astral.sh/ruff/rules/raise-within-try/).

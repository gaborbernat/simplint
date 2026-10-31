Ruff's cleanup after a fix keeps every rule on and only limits which ones it fixes, so it no longer drops suppression
comments such as `# ruff: ignore[unused-import]` as unused and then removes the imports they kept for other modules.

`blanket-ignore` (SMP004) and `suppression-without-reason` (SMP003) read `noqa` comments as Ruff does. A `# noqa:` whose
colon names no code, a code that runs into other text, a `# ruff: noqa` after code on its line and a
`# ruff: file-ignore` below module scope suppress nothing in Ruff, so they are no longer reported. A `noqa` comment
silences simplint's errors by the same reading.

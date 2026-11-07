`unused-definition` (SMP409) no longer reports the hook class of a hatchling custom build or metadata hook, such as
`_CustomBuildHook` in `hatch_build.py`, as unused. hatchling calls that class, and the `path` of the hook is honored.

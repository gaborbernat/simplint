`unused-export` (SMP402) no longer reports names in the `__all__` of a public module of a library, since users import
those names and no module of the project needs to. Private modules and applications keep the report.

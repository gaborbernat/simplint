A file that imports a module of the run with `import module` shares that module's members with the index instead of
copying them for each importer: 8,000 modules that import a module with 8,000 functions take 0.16 seconds and 128 MB
instead of 2.2 seconds and 4.9 GB.

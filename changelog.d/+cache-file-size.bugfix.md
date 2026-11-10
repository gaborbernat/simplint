The cache keys each file by its size as well as its modification time and permissions, so a file rewritten within one
tick of a coarse clock, or copied with its modification time kept (`cp -p`, `rsync -t`), is checked again instead of
reported from its old cached result.

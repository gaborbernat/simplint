A cached run lists the directories under each project and search root, to tell whether the documentation or the import
layout changed, on all threads instead of one, so a project with a large `node_modules/` or gitignored `target/` tree no
longer spends most of a cached run walking it: with 20,000 such directories a fully cached run takes 0.14 seconds
instead of 0.22.

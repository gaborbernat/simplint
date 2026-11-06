`up-compression-modules` (SMP523) reports imports of `gzip`, `bz2`, `lzma` and `zlib` from Python 3.14, where the
`compression` package names them. The fix is unsafe: `compression` re-exports the modules, so patches of the old module
names do not reach the new ones.

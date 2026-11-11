Looking up what an imported name refers to no longer scans all of the importing file's imports for each name, so a
module of 64,000 top-level `from` imports takes 0.56 seconds instead of 13.9.

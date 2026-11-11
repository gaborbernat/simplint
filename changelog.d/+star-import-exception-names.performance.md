A module that star-imports other modules no longer walks the whole chain of star imports behind it again for each
importer when looking for exception classes: 16,000 modules, each star-importing the one before, take 0.76 seconds
instead of 3.8.

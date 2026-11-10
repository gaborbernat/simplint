Following a name through a chain of modules that re-export it remembers where each module leads, so every module of the
chain no longer walks the rest of it: 8,000 packages that each re-export the name of the one before take 0.2 seconds
instead of 4.6.

`module-attribute-import` (SMP304) no longer reports an attribute read off a parameter, local, loop or comprehension
variable named like the module it imports. Only the reads that resolve to the module are reported, and its fix rewrites
only those reads. A module whose name a local, parameter, closure variable or class attribute rebinds used to be skipped
entirely; its real reads are now reported and fixed.

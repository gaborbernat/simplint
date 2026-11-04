`string-concatenation` (SMP112) no longer slows to a crawl on a function where many names each extend
themselves in turn, as `text = text + "!"` does; such chains now take time in proportion to their length.

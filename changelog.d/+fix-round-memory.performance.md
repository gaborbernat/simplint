`--fix` runs that rename names across a large project use less memory: the words of each file kept between fix rounds
now take about 8 bytes each, which cut peak memory from 830 MB to 474 MB on 3,000 word-heavy modules.

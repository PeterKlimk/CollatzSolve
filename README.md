# Collatz Finder
Concurrent program to find the Collatz chain with the smallest starting number, out of chains with delays greater than some target number. Efficient, as it uses multiple threads and the precomputation trick to skip a large number of steps at a time.

Example, to find a collatz chain with a delay > 1300. It will use 4 threads, and a few GB of memory as the precomputation is set to 26, and the cache size is set to 2<sup>(26+4)</sup>=2<sup>30</sup> numbers.
```
collatz --target 1300 --threads 4 --iter 26 --cache 4
```

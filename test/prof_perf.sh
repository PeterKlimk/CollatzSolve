cargo build --release
perf record -g ../target/release/collatz --target 1040 --threads 3 --iter 19 --cache 5 --block=100000000
perf script | ./stackcollapse-perf.pl | ./rust-unmangle | ./flamegraph.pl > flame.svg
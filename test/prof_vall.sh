cargo build --release
valgrind --tool=callgrind ../target/release/collatz --target 900 --threads 2 --iter 17 --cache 6 --block=100000000
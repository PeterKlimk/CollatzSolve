#![feature(alloc_system, integer_atomics)]

#[macro_use]
extern crate clap;
extern crate alloc_system;
extern crate time;

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;

use time::PreciseTime;

use clap::{App, Arg};

#[derive(Debug, Clone, Copy)]
struct Output {
    start: i128,
    count: i16,
}

impl Output {
    fn is_better_than(&self, other: &Output) -> bool {
        (self.count > other.count) || (self.count == other.count && self.start > other.start)
    }
}

struct ThreadOutput {
    output: Output,
    block: i64,
}

struct Problem {
    iterations: i16,
    pow: i64,
    cache_size: usize,
    cache: Vec<i16>,
    odds: Vec<i16>,
    c: Vec<i64>,
    d: Vec<i64>,
}

impl Problem {
    fn _gen_preload(&mut self) {
        for b in 0i64..self.pow {
            let mut odd: i16 = 0;
            let mut n: i64 = b;

            for _ in 0..self.iterations {
                if n % 2 == 0 {
                    n = n / 2;
                } else {
                    odd += 1;
                    n = (3 * n + 1) / 2;
                }
            }

            self.d.push(n);
            self.c.push(i64::pow(3, odd as u32));
            self.odds.push(odd);
        }
    }

    fn _gen_cache(cache_size: usize) -> Vec<i16> {
        let mut cache = vec![0i16; cache_size];
        cache[1] = 0;

        for i in 2i128..(cache_size as i128) {
            let mut current: i128 = i;
            let mut count: i16 = 0;

            loop {
                if current % 2 == 0 {
                    current = current / 2;
                } else {
                    current = current * 3 + 1;
                }

                count += 1;

                if current < i {
                    count += cache[current as usize];
                    break;
                }
            }

            cache[i as usize] = count as i16;
        }

        cache
    }

    fn generate(iterations: i16, extra_cache: i16) -> Self {
        let pow = 2i64.pow(iterations as u32);
        let cache_size = 2i64.pow((iterations + extra_cache) as u32) as usize;

        let mut problem = Self {
            pow: pow,
            iterations: iterations,
            cache_size: cache_size,
            cache: Self::_gen_cache(cache_size),
            odds: Vec::new(),
            c: Vec::new(),
            d: Vec::new(),
        };

        problem._gen_preload();

        problem
    }

    fn solve(self, target: i16, block_size: i64, thread_limit: usize) -> Output {
        let arc = Arc::new(self);
        let (tx, rx): (Sender<ThreadOutput>, Receiver<ThreadOutput>) = mpsc::channel();

        let atomic_block = Arc::new(AtomicI64::new(0));

        let mut best_output = Output { start: 0, count: 0 };
        let output_lock = Arc::new(RwLock::new(best_output));

        let mut outputs = HashMap::new();

        for _ in 0..thread_limit {
            let thread_arc = Arc::clone(&arc);
            let thread_atomic = Arc::clone(&atomic_block);
            let thread_tx = tx.clone();
            let thread_output = output_lock.clone();

            thread::spawn(move || loop {
                let block = thread_atomic.fetch_add(1, Ordering::SeqCst);
                let current_best = *thread_output.read().unwrap();

                let result = thread_arc._solve(
                    target,
                    (block * block_size) as i128,
                    ((block + 1) * block_size) as i128,
                    current_best,
                );
                let t = thread_tx.send(ThreadOutput {
                    output: result,
                    block: block,
                });

                if let Err(_) = t {
                    break;
                }
            });
        }

        loop {
            let thread_output = rx.recv().unwrap();

            outputs.insert(thread_output.block, thread_output.output);

            let mut i = 0;
            loop {
                match outputs.get(&i) {
                    Some(result) => {
                        if result.count > target {
                            return *result;
                        } else if result.is_better_than(&best_output) {
                            best_output = thread_output.output;
                            let mut w = output_lock.write().unwrap();
                            *w = best_output;
                        }
                    }
                    None => break,
                }

                i += 1;
            }
        }
    }

    fn _solve(&self, target: i16, min: i128, max: i128, mut best_output: Output) -> Output {
        let mut limit = 0;
        let mut peak_count = 0;

        let mut n: i128;
        let mut double_step: bool;

        match min % 3 {
            0 => {
                double_step = true;
                n = min;
            }
            1 => {
                double_step = false;
                n = min + 1;
            }
            2 => {
                double_step = false;
                n = min;
            }
            _ => panic!("Somehow, min mod 3 is not 0, 1 or 2."),
        }

        while n < max {
            let mut current: i128 = n;
            let mut count: i16 = 0;

            while current >= self.cache_size as i128 {
                let a: i128 = current >> self.iterations;
                let b: i128 = current & ((1 << self.iterations) - 1);

                current = a * (self.c[b as usize] as i128) + (self.d[b as usize] as i128);
                count += self.iterations;
                count += self.odds[b as usize];

                if current < best_output.start && (count + best_output.count) <= target {
                    break;
                }
            }

            if current < self.cache_size as i128 {
                count += self.cache[current as usize];

                if count >= best_output.count {
                    best_output.start = n;
                    best_output.count = count;

                    if count > target {
                        return best_output;
                    }
                }

                if count > peak_count {
                    peak_count = count;
                    limit = n * 2;
                }
            }
            
            loop {
                if double_step {
                    n += 2;
                    double_step = false;
                } else {
                    n += 1;
                    double_step = true;
                }

                if n % 2 == 0 {
                    if n >= limit {
                        break;
                    }
                } else if double_step && ((peak_count + 3) > target || n % 4 != 1) {
                    break;
                }
            }
        }

        return best_output;
    }
}

fn main() {
    let matches = App::new("Collatz Finder")
        .version("1.0")
        .author("Peter Klimenko <peterklimk@gmail.com>")
        .about("Finds the collatz sequence with the smallest starting number, out of sequences with delays greater than your target.")
        .arg(Arg::with_name("target")
            .help("The target delay. Output will be the smallest collatz")
            .short("t")
            .long("target")
            .value_name("TARGET")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("threads")
            .help("The number of threads.")
            .short("c")
            .long("threads")
            .value_name("THREADS")
            .takes_value(true)
            .default_value("4"))
        .arg(Arg::with_name("iterations")
            .help("The number of collatz steps to. More is faster but takes exponentially more resources.")
            .short("i")
            .long("iter")
            .value_name("ITERATIONS")
            .takes_value(true)
            .default_value("20"))
        .arg(Arg::with_name("extra_cache")
            .help("The cache is normally of size 2^iterations. This argument adds extra cache, by making the cache size 2^(iterations + z).")
            .short("z")
            .long("cache")
            .value_name("EXTRA CACHE LAYERS")
            .takes_value(true)
            .default_value("0"))
        .arg(Arg::with_name("block_size")
            .help("Sets the size of the block of numbers given to threads. Smaller blocks result in more thread turnover, but blocks that are too large may not efficiently allocate resources.")
            .short("b")
            .long("block")
            .value_name("BLOCK SIZE")
            .takes_value(true)
            .default_value("1000000000"))
        .get_matches();

    let target = value_t!(matches, "target", i16).unwrap();
    let thread_limit = value_t!(matches, "threads", usize).unwrap();
    let iterations = value_t!(matches, "iterations", i16).unwrap();
    let block_size = value_t!(matches, "block_size", i64).unwrap();
    let extra_cache = value_t!(matches, "extra_cache", i16).unwrap();

    println!("STARTING");
    println!("________");

    let start = PreciseTime::now();

    let problem = Problem::generate(iterations, extra_cache);

    println!("Generation: {} seconds.", start.to(PreciseTime::now()));
    let middle = PreciseTime::now();
    let output = problem.solve(target, block_size, thread_limit);

    println!("Solving: {} seconds.", middle.to(PreciseTime::now()));
    println!("Total: {} seconds.", start.to(PreciseTime::now()));

    println!("________");
    println!("RESULT");
    println!("________");
    println!("Number: {}", output.start);
    println!("Count: {}", output.count);
}

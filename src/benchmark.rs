use crate::fast_bvec::*;
use crate::bvec::*;
use crate::tst;
use crate::tst::ExecQueries;
use rand::Rng;
use seq_macro::seq;
use prettytable::*;
use memuse::DynamicUsage;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use rand::seq::SliceRandom;
use colored::Colorize;

pub struct Params<const A: usize, const B: usize, const C: usize = 1, const SUPERBITS: usize = 40>;

impl<const A: usize, const B: usize, const C: usize, const D: usize> RASBVecParameters for Params<A, B, C, D> {
    const BLOCK_SIZE: usize = A;
    const SUPERBLOCK_SIZE: usize = B;
    const MEGABLOCK_FACTOR: usize = C;
    const SUPERBLOCK_BITS: usize = D;
}

#[macro_export]
macro_rules! measure_time {
    ($block:block) => {
        {
            let start = std::time::Instant::now();
            $block
            start.elapsed().as_nanos().div_floor(1000000)
        }
    }
}

trait Benchmarker {
    fn init_benchmark(&mut self, bitlen: usize) -> BitVector {
        BitVector::generate_random(bitlen, 1)
    }

    fn run_benchmark<I: RankSelectVector>(&self, bv: &I) where Self: Sized;
}

fn benchmark_generic_random<Bench: Benchmarker>(bitlen: usize, mut b: Bench) {
    const BLOCKS: [usize; 8] = [256, 512, 1024, 2048, 4096, 8192, 16384, 32768];
    const SUPERBLOCKS: [usize; 6] = [4096, 8192, 16384, 32768, 65536, 131072];

    const N: usize = BLOCKS.len();
    const M: usize = SUPERBLOCKS.len();

    let mut build_times = vec![vec![0u128; M]; N];
    let mut memory = vec![vec![0u128; M]; N];
    let mut runtimes = vec![vec![0u128; M]; N];
    let bits = b.init_benchmark(bitlen);

    seq!(I in 0..8 {
        seq!(J in 0..6 {
            {
                const BLOCK_SIZE: usize = BLOCKS[I];
                const SUPERBLOCK_SIZE: usize = SUPERBLOCKS[J];

                if BLOCK_SIZE <= SUPERBLOCK_SIZE {
                    type AccelVector = FastRASBVec<Params<BLOCK_SIZE, SUPERBLOCK_SIZE, 32>>;
                    let mut bv = AccelVector::new_empty();
                    let bclone = bits.clone();
                    build_times[I][J] = measure_time!({
                        bv.initialize_for(bclone);
                    });

                    memory[I][J] = bv.dynamic_usage() as u128 + std::mem::size_of::<AccelVector>() as u128;
                    runtimes[I][J] = measure_time!({
                        b.run_benchmark(&bv);
                    });

                    println!("Finished B={} S={} build={}ms run={}ms", BLOCK_SIZE, SUPERBLOCK_SIZE, build_times[I][J], runtimes[I][J]);
                }
            }
        });
    });

    let mut table_build = Table::new();
    let mut table_runtime = Table::new();

    let mut header = Row::empty();
    header.add_cell(Cell::new("B\\S"));
    for i in SUPERBLOCKS {
        header.add_cell(Cell::new(format!("{}", i).as_str()));
    }
    table_build.add_row(header.clone());
    table_runtime.add_row(header);

    for i in 0..BLOCKS.len() {
        let mut line_build = Row::empty();
        let mut line_run = Row::empty();

        line_build.add_cell(Cell::new(format!("{}", BLOCKS[i]).as_str()));
        line_run.add_cell(Cell::new(format!("{}", BLOCKS[i]).as_str()));

        for j in 0..SUPERBLOCKS.len() {
            if BLOCKS[i] > SUPERBLOCKS[j] {
                line_run.add_cell(Cell::new("-------"));
                line_build.add_cell(Cell::new("----------------"));
            } else {
                line_run.add_cell(Cell::new(format!("{}ms", runtimes[i][j]).as_str()));
                line_build.add_cell(Cell::new(format!("{}ms / {:.2} MB",
                                                      build_times[i][j],
                                                      memory[i][j] as f64 / 1024.0 / 1024.0).as_str()));
            }
        }

        table_build.add_row(line_build);
        table_runtime.add_row(line_run);
    }
    println!("Bit vector space: {:.2} MB", bits.dynamic_usage() as f64 / 1024.0 / 1024.0);

    println!("Build times:");
    table_build.printstd();

    println!("Run times:");
    table_runtime.printstd();
}

struct RankBenchmark {
    queries: Vec<usize>
}

impl Benchmarker for RankBenchmark {
    fn run_benchmark<I: RankSelectVector>(&self, bv: &I) where Self: Sized {
        for x in &self.queries {
            bv.rank(*x);
        }

    }
}

pub fn benchmark_rank(n: usize, q: usize) {
    let n = n;
    let mut rng = Xoshiro256Plus::seed_from_u64(123);

    let mut bench = RankBenchmark {
        queries: (0..q).collect::<Vec<_>>()
    };

    bench.queries.shuffle(&mut rng);
    benchmark_generic_random(n, bench);
}

struct RandomSelectBenchmark {
    queries: Vec<(usize, bool)>,
    nr_queries: usize,
    seed: u64,
}

impl RandomSelectBenchmark {
    fn new(nr_queries: usize, seed: u64) -> Self {

        Self {
            queries: Vec::new(),
            nr_queries,
            seed,
        }
    }
}

fn generate_random_select_queries(bits: &BitVector, nr_queries: usize, seed: u64) -> Vec<(usize, bool)> {
    let count1 = bits.count_ones(0, bits.size());
    let count0 = bits.size() - count1;

    let mut rng = Xoshiro256Plus::seed_from_u64(seed);
    (0..nr_queries).map(|_| {
        let t = rng.gen_bool(0.5);
        let pos = if t { count1 } else { count0 };
        let x = rng.gen_range(1..=pos);
        (x, t)
    }).collect::<Vec<_>>()
}

impl Benchmarker for RandomSelectBenchmark {
    fn init_benchmark(&mut self, bitlen: usize) -> BitVector {
        let bits = BitVector::generate_random(bitlen, 33333);
        self.queries = generate_random_select_queries(&bits, self.nr_queries, self.seed);
        bits
    }

    fn run_benchmark<I: RankSelectVector>(&self, bv: &I) where Self: Sized {
        for (x, t) in &self.queries {
            if *t {
                bv.select1(*x);
            } else {
                bv.select0(*x);
            }
        }
    }
}

pub fn benchmark_select_bruteforce_param(n: usize, queries: usize) {
    const BRUTEFORCE: [usize; 10] = [1, 2, 4, 8, 16, 32, 128, 256, 512, 1024];
    const N: usize = BRUTEFORCE.len();

    let mut runtimes0 = vec![0u128; N];
    let mut runtimes1 = vec![0u128; N];

    let bv = BitVector::generate_random(n, 4444);
    let queries = generate_random_select_queries(&bv, queries, 111);

    seq!(I in 0..10 {
        {
            const BR: usize = BRUTEFORCE[I];

            let bits = bv.clone();
            type AccelVector = FastRASBVec<Params<8192, 32768, BR>>;
            let bv = AccelVector::new(bits);

            runtimes1[I] = measure_time!({
                for (x, t) in &queries {
                    if *t {
                        bv.select1(*x);
                    }
                }
            });

            runtimes0[I] = measure_time!({
                for (x, t) in &queries {
                    if !*t {
                        bv.select0(*x);
                    }
                }
            });

            println!("Finished BRUTE={} run1={}ms run0={}ms", BR, runtimes1[I], runtimes0[I]);
        }
    });

    let mut table = Table::new();
    let mut header = Row::empty();
    header.add_cell(Cell::new("Bruteforce"));
    header.add_cell(Cell::new("Run 1"));
    header.add_cell(Cell::new("Run 0"));
    header.add_cell(Cell::new("Total"));

    table.add_row(header);

    for i in 0..BRUTEFORCE.len() {
        let mut line = Row::empty();

        line.add_cell(Cell::new(format!("{}", BRUTEFORCE[i]).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", runtimes1[i] as f64 / 1000.0).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", runtimes0[i] as f64 / 1000.0).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", (runtimes1[i] + runtimes0[i]) as f64 / 1000.0).as_str()));
        table.add_row(line);
    }

    table.printstd();
}

struct RandomRankSelectBenchmark {
    queries: Vec<tst::Query>,
    nr_queries: usize,
    seed: u64,
}

impl RandomRankSelectBenchmark {
    fn new(nr_queries: usize, seed: u64) -> Self {
        Self {
            queries: Vec::new(),
            nr_queries,
            seed,
        }
    }
}

impl Benchmarker for RandomRankSelectBenchmark {
    fn init_benchmark(&mut self, bitlen: usize) -> BitVector {
        let bits = BitVector::generate_random(bitlen, 33333);
        let ones = bits.count_ones(0, bitlen);
        self.queries = tst::generate_random_queries(self.nr_queries, self.seed, self.nr_queries, Some(ones));
        bits
    }

    fn run_benchmark<I: RankSelectVector>(&self, bv: &I) where Self: Sized {
        self.queries.iter().exec_queries(bv).for_each(drop);
    }
}

#[allow(dead_code)]
pub enum AllBench {
    Random,
    SelectBruteforce,
    SelectGeneral,
    RankGeneral,
}

pub fn benchmark_select_all(list: &[AllBench]) {
    let q = 1 << 23;
    let n = 1usize << 34;

    for l in list.iter() {
        match l {
            AllBench::Random => {
                println!("{}", "Testing rank and select with general bit vector".blue().bold());
                let bench = RandomRankSelectBenchmark::new(q, 122);
                benchmark_generic_random(n, bench);
            }
            AllBench::SelectGeneral => {
                println!("{}", "Testing select with random bit vector".blue().bold());
                let random = RandomSelectBenchmark::new(q, 111);
                benchmark_generic_random(n, random);
            },
            AllBench::SelectBruteforce => {
                println!("{}", "Testing select bruteforce param".blue().bold());
                benchmark_select_bruteforce_param(n, q);
            }
            AllBench::RankGeneral => {
                println!("{}", "Testing rank with random bit vector".blue().bold());
                benchmark_rank(n, q);
            }

        }
    }
}

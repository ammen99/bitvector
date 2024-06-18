use crate::fast_bvec::*;
use crate::bvec::*;
use crate::tst::SectionDescription;
use rand::Rng;
use seq_macro::seq;
use crate::tst;
use prettytable::*;
use memuse::DynamicUsage;
use rand::SeedableRng;
use rand_xoshiro::Xoshiro256Plus;
use rand::seq::SliceRandom;
use colored::Colorize;

struct Params<const A: usize, const B: usize, const C: usize, const D: usize = 2>;

impl<const A: usize, const B: usize, const C: usize, const D: usize> RASBVecParameters for Params<A, B, C, D> {
    const BLOCK_SIZE: usize = A;
    const SUPERBLOCK_SIZE: usize = B;
    const SELECT_SUPERBLOCK: usize = C;
    const SELECT_BRUTEFORCE: usize = D;
}

macro_rules! measure_time {
    ($block:block) => {
        {
            let start = std::time::Instant::now();
            $block
            start.elapsed().as_nanos().div_floor(1000000)
        }
    }
}

#[allow(dead_code)]
pub fn benchmark_rank() {
    const BLOCKS: [usize; 3] = [64, 256, 1024];
    const SUPERBLOCKS: [usize; 6] = [512, 1024, 4096, 8192, 16384, 32768];

    const N: usize = BLOCKS.len();
    const M: usize = SUPERBLOCKS.len();

    let mut build_times = vec![vec![0u128; M]; N];
    let mut memory = vec![vec![0u128; M]; N];
    let mut runtimes = vec![vec![0u128; M]; N];

    let string = tst::generate_random_bits_string(1 << 26, 1, 0.5);

    let mut rng = Xoshiro256Plus::seed_from_u64(123);
    let mut queries = (0..(1 << 26)).collect::<Vec<_>>();
    queries.shuffle(&mut rng);

    seq!(I in 0..3 {
        seq!(J in 0..6 {
            {
                const BLOCK_SIZE: usize = BLOCKS[I];
                const SUPERBLOCK_SIZE: usize = SUPERBLOCKS[J];

                if BLOCK_SIZE <= SUPERBLOCK_SIZE {
                    let bits = BitVector::new_from_string(&string);
                    type AccelVector = FastRASBVec<Params<BLOCK_SIZE, SUPERBLOCK_SIZE, 10000000000>>;
                    let mut bv = AccelVector::new_empty();
                    build_times[I][J] = measure_time!({
                        bv.initialize_for(bits);
                    });

                    memory[I][J] = bv.dynamic_usage() as u128 + std::mem::size_of::<AccelVector>() as u128;


                    runtimes[I][J] = measure_time!({
                        for x in &queries {
                            bv.rank(*x);
                        }
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
                line_run.add_cell(Cell::new(format!("{:.3}s", runtimes[i][j] as f64 / 1000.0).as_str()));
                line_build.add_cell(Cell::new(format!("{:.3}s / {:.2} MB",
                                                      build_times[i][j] as f64 / 1000.0,
                                                      memory[i][j] as f64 / 1024.0 / 1024.0).as_str()));
            }
        }

        table_build.add_row(line_build);
        table_runtime.add_row(line_run);
    }

    println!("Build times:");
    table_build.printstd();

    println!("Run times:");
    table_runtime.printstd();
}

pub fn generate_random_select(pattern: &[tst::SectionDescription], pattern_repeat: usize, n_queries: usize) -> (String, Vec<(usize, bool)>) {
    let string = tst::generate_random_bits_in_sections(pattern, pattern_repeat, 124);

    let count1 = string.bytes().filter(|x| *x == b'1').count();
    let count0 = string.len() - count1;

    let mut rng = Xoshiro256Plus::seed_from_u64(123);
    let queries = (0..n_queries).map(|_| {
        let t = rng.gen_bool(0.5);
        let pos = if t { count1 } else { count0 };
        let x = rng.gen_range(1..=pos);
        (x, t)
    }).collect::<Vec<_>>();

    (string, queries)
}

pub fn benchmark_select_one(pattern: &[tst::SectionDescription], pattern_repeat: usize, n_queries: usize) {
    const SELECT_BLOCKS: [usize; 7] = [4, 16, 64, 256, 1024, 4096, 16384];
    const N: usize = SELECT_BLOCKS.len();

    let mut build_times = vec![0u128; N];
    let mut memory = vec![0u128; N];
    let mut runtimes0 = vec![0u128; N];
    let mut runtimes1 = vec![0u128; N];
    let (string, queries) = generate_random_select(pattern, pattern_repeat, n_queries);

    seq!(I in 0..7 {
        {
            const SUPER: usize = SELECT_BLOCKS[I];

            let bits = BitVector::new_from_string(&string);
            type AccelVector = FastRASBVec<Params<256, 4096, SUPER>>;
            let mut bv = AccelVector::new_empty();
            build_times[I] = measure_time!({
                bv.initialize_for(bits);
            });

            memory[I] = bv.dynamic_usage() as u128 + std::mem::size_of::<AccelVector>() as u128;


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

            println!("Finished SELECT_BLOCK={} build={}ms run1={}ms run0={}ms", SUPER, build_times[I], runtimes1[I], runtimes0[I]);
        }
    });

    let mut table = Table::new();
    let mut header = Row::empty();
    header.add_cell(Cell::new(""));
    header.add_cell(Cell::new("Build"));
    header.add_cell(Cell::new("Space"));
    header.add_cell(Cell::new("Run 1"));
    header.add_cell(Cell::new("Run 0"));
    header.add_cell(Cell::new("Total"));

    table.add_row(header);

    for i in 0..SELECT_BLOCKS.len() {
        let mut line = Row::empty();

        line.add_cell(Cell::new(format!("{}", SELECT_BLOCKS[i]).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", build_times[i] as f64 / 1000.0).as_str()));
        line.add_cell(Cell::new(format!("{:.2} MB", memory[i] as f64 / 1024.0 / 1024.0).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", runtimes1[i] as f64 / 1000.0).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", runtimes0[i] as f64 / 1000.0).as_str()));
        line.add_cell(Cell::new(format!("{:.3}s", (runtimes1[i] + runtimes0[i]) as f64 / 1000.0).as_str()));
        table.add_row(line);
    }

    table.printstd();
}

pub fn benchmark_select_bruteforce_param() {
    const BRUTEFORCE: [usize; 8] = [32, 128, 512, 1024, 4096, 8192, 16384, 32768];
    const N: usize = BRUTEFORCE.len();

    let mut runtimes0 = vec![0u128; N];
    let mut runtimes1 = vec![0u128; N];

    let section = 1 << 20;
    let mixed = [SectionDescription{weight0: 0.01, section_len: section, probability: 1.0},
        SectionDescription{weight0: 0.5, section_len: section, probability: 1.0}];
    let (string, queries) = generate_random_select(&mixed, 16, 1 << 20);

    seq!(I in 0..8 {
        {
            const BR: usize = BRUTEFORCE[I];

            let bits = BitVector::new_from_string(&string);
            type AccelVector = FastRASBVec<Params<256, 4096, 262144, BR>>;
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

#[allow(dead_code)]
pub enum AllBench {
    Random,
    Sparse,
    Mixed,
    Bruteforce,
}

pub fn benchmark_select_all(list: &[AllBench]) {
    let q = 1 << 20;
    let n = 1 << 25;

    for l in list.iter() {
        match l {
            AllBench::Random => {
                let random = [SectionDescription{weight0: 0.5, section_len: n, probability: 1.0}];
                println!("{}", "Testing select with random bit vector".blue().bold());
                benchmark_select_one(&random, 1, q);
            },

            AllBench::Sparse => {
                let sparse = [SectionDescription{weight0: 0.01, section_len: n, probability: 1.0}];
                println!("{}", "Testing select with sparse bit vector".blue().bold());
                benchmark_select_one(&sparse, 1, q);

            },
            AllBench::Mixed => {
                let section = 1 << 16;
                let mixed = [SectionDescription{weight0: 0.01, section_len: section, probability: 1.0},
                SectionDescription{weight0: 0.5, section_len: section, probability: 1.0}];
                println!("{}", "Testing select with mixed bit vector".blue().bold());
                benchmark_select_one(&mixed, n / section, q);
            },
            AllBench::Bruteforce => {
                println!("{}", "Testing select bruteforce param".blue().bold());
                benchmark_select_bruteforce_param();
            }
        }
    }
}

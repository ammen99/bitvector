use crate::fast_bvec::*;
use crate::bvec::*;
use seq_macro::seq;
use crate::tst;
use prettytable::*;
use memuse::DynamicUsage;

struct Params<const A: usize, const B: usize, const C: usize>;

impl<const A: usize, const B: usize, const C: usize> RASBVecParameters for Params<A, B, C> {
    const BLOCK_SIZE: usize = A;
    const SUPERBLOCK_SIZE: usize = B;
    const SELECT_SUPERBLOCK: usize = C;
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

pub fn benchmark_rank() {
    const BLOCKS: [usize; 5] = [4, 64, 256, 1024, 4096];
    const SUPERBLOCKS: [usize; 3] = [512, 1024, 4096];

    const N: usize = BLOCKS.len();
    const M: usize = SUPERBLOCKS.len();
    const BASE: u128 = 136395369829;

    let mut build_times = vec![vec![0u128; M]; N];
    let mut memory = vec![vec![0u128; M]; N];
    let mut runtimes = vec![vec![0u128; M]; N];

    let string = tst::generate_random_bits_string(1 << 23, 1, 0.5);

    seq!(I in 0..5 {
        seq!(J in 0..3 {
            {
                const BLOCK_SIZE: usize = BLOCKS[I];
                const SUPERBLOCK_SIZE: usize = SUPERBLOCKS[J];

                if BLOCK_SIZE <= SUPERBLOCK_SIZE {
                    let bits = BitVector::new_from_string(&string);
                    type AccelVector = FastRASBVec<Params<BLOCK_SIZE, SUPERBLOCK_SIZE, 10000000000>>;
                    let mut bv = AccelVector::new_empty();

                    memory[I][J] = bv.dynamic_usage() as u128 + std::mem::size_of::<AccelVector>() as u128;

                    build_times[I][J] = measure_time!({
                        bv.initialize_for(bits);
                    });

                    runtimes[I][J] = measure_time!({
                        for i in 0..string.len() {
                            bv.rank(((i as u128 * BASE) % (string.len() as u128)) as usize);
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
            line_run.add_cell(Cell::new(format!("{:.3} ms", runtimes[i][j] as f64 / 1000.0).as_str()));
            line_build.add_cell(Cell::new(format!("{:.3} ms / {:.2} MB",
                                                  build_times[i][j] as f64 / 1000.0,
                                                  memory[i][j] as f64 / 1024.0 / 1024.0).as_str()));
        }

        table_build.add_row(line_build);
        table_runtime.add_row(line_run);
    }

    println!("Build times:");
    table_build.printstd();

    println!("Run times:");
    table_runtime.printstd();
}

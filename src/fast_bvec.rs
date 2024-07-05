use num::Integer;

use crate::bvec::*;
use memuse::DynamicUsage;


macro_rules! num_blocks {
    ($size:expr) => {
        (($size - std::mem::size_of::<usize>()) / 2)
    };
}

#[derive(Debug, Clone)]
struct RankSuperblock<const CACHELINE_SIZE: usize> where [u16; num_blocks!(CACHELINE_SIZE)]: Sized {
    before: usize,
    blocks: [u16; num_blocks!(CACHELINE_SIZE)],
}

impl<const CACHELINE_SIZE: usize> RankSuperblock<CACHELINE_SIZE> where [u16; num_blocks!(CACHELINE_SIZE)]: Sized {
    fn new() -> Self {
        RankSuperblock::<CACHELINE_SIZE> {
            before: 0,
            blocks: [0; num_blocks!(CACHELINE_SIZE)],
        }
    }
}

impl<const CACHELINE_SIZE: usize> DynamicUsage for RankSuperblock<CACHELINE_SIZE> where [u16; num_blocks!(CACHELINE_SIZE)]: Sized {
    fn dynamic_usage(&self) -> usize {
        0
    }

    fn dynamic_usage_bounds(&self) -> (usize, Option<usize>) {
        (self.dynamic_usage(), None)
    }
}

struct RankSupport<const CACHELINE_SIZE: usize> where [u16; num_blocks!(CACHELINE_SIZE)]: Sized {
    superblocks: Vec<RankSuperblock<CACHELINE_SIZE>>,
}

pub trait RASBVecParameters {
    const BLOCK_SIZE: usize;
    const SUPERBLOCK_SIZE: usize;
    const SELECT_BRUTEFORCE: usize = 2;
    const CACHELINE_SIZE: usize = ((Self::SUPERBLOCK_SIZE / Self::BLOCK_SIZE) * 2 + 8).next_multiple_of(1);
}

pub struct FastRASBVec<Parameters: RASBVecParameters> where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {
    bits: BitVector,
    rank: RankSupport<{Parameters::CACHELINE_SIZE}>,
    count0: usize,
    count1: usize,
    pd: std::marker::PhantomData<Parameters>,
}

#[allow(dead_code)]
impl<Parameters: RASBVecParameters> FastRASBVec<Parameters> where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {
    pub fn size(&self) -> usize {
        self.bits.size()
    }

    pub fn new_empty() -> Self {
        FastRASBVec::<Parameters> {
            bits: BitVector::new_from_string("0"),
            rank: RankSupport {
                superblocks: vec![],
            },
            count0: 0,
            count1: 0,
            pd: std::marker::PhantomData,
        }
    }

    pub fn initialize_for(&mut self, bits: BitVector) {
        self.bits = bits;
        let (rank, count1) = Self::init_rank(&self.bits);
        self.rank = rank;
        self.count0 = self.bits.size() - count1;
        self.count1 = count1;
    }

    pub fn debug_print(&self) {
        println!("Superblocks: {:?}", self.rank.superblocks);
        println!("Count0: {}", self.count0);
        println!("Count1: {}", self.count1);
    }

    pub fn blocks_per_superblock() -> usize {
        Parameters::SUPERBLOCK_SIZE / Parameters::BLOCK_SIZE
    }

    fn init_rank(bits: &BitVector) -> (RankSupport<{Parameters::CACHELINE_SIZE}>, usize) {
        let n_super = bits.size().div_ceil(Parameters::SUPERBLOCK_SIZE);

        let mut rk = RankSupport {
            superblocks: vec![RankSuperblock::new(); n_super],
        };

        let mut total_count: usize = 0;
        for i in 0..n_super {
            let mut sblock_count: u16 = 0;
            assert!(usize::from(u16::MAX) >= (Self::blocks_per_superblock() - 1) * Parameters::BLOCK_SIZE, "Superblock size is too big for u16 blocks!");
            assert!(Self::blocks_per_superblock() <= rk.superblocks[i].blocks.len(), "Try increasing cache line size!");

            rk.superblocks[i].before = total_count;
            for j in 0..Self::blocks_per_superblock() {
                rk.superblocks[i].blocks[j] = sblock_count;

                let block_start = i * Parameters::SUPERBLOCK_SIZE + j * Parameters::BLOCK_SIZE;
                if block_start < bits.size() {
                    let block_end = std::cmp::min(block_start + Parameters::BLOCK_SIZE, bits.size());
                    sblock_count += bits.count_ones(block_start, block_end) as u16;
                }
            }

            total_count += sblock_count as usize;
        }

        (rk, total_count)
    }

    fn _rank1(&self, i: usize) -> usize {
        let (super_idx, super_rem) = i.div_rem(&Parameters::SUPERBLOCK_SIZE);
        let (block_idx, block_rem) = super_rem.div_rem(&Parameters::BLOCK_SIZE);

        //println!("super_idx: {}, super_rem: {}, block_idx: {}, block_rem: {}", super_idx, super_rem, block_idx, block_rem);

        let mut r = self.rank.superblocks[super_idx].before;
        r += self.rank.superblocks[super_idx].blocks[block_idx] as usize;
        r += self.bits.count_ones(i - block_rem, i);
        r
    }

    fn generic_rank(&self, i: usize, value: u32) -> usize {
        let r = self._rank1(i);
        if value == 1 {
            r
        } else {
            i - r
        }
    }

    fn value_count_before_sblock(&self, b: usize, value: u32) -> usize {
        if value == 1 {
            self.rank.superblocks[b].before
        } else {
            b * Parameters::SUPERBLOCK_SIZE - self.rank.superblocks[b].before
        }
    }

    fn value_count_before_block(&self, sb: usize, b: usize, value: u32) -> usize {
        if value == 1 {
            self.rank.superblocks[sb].blocks[b] as usize
        } else {
            b * Parameters::BLOCK_SIZE - self.rank.superblocks[sb].blocks[b] as usize
        }
    }

    fn generic_select(&self, i: usize, value: u32) -> Option<usize> {
        if i == 0 {
            return None;
        }

        let total = if value == 0 { &self.count0 } else { &self.count1 };
        if i > *total {
            return None
        }

        // Step 1: binary search over rank superblocks, so that we can find the superblock where
        // our match should be.
        let mut lsblock = 0usize;
        let mut rsblock = self.rank.superblocks.len();
        while rsblock - lsblock > Parameters::SELECT_BRUTEFORCE {
            let mid = (lsblock + rsblock) / 2;
            if self.value_count_before_sblock(mid, value) >= i {
                rsblock = mid;
            } else {
                lsblock = mid;
            }
        }

        // Finish the search, because we do the last few blocks with manual search, benchmarks show
        // it is faster this way.
        while self.value_count_before_sblock(rsblock - 1, value) >= i {
            rsblock -= 1;
        }

        let start_sblock = rsblock - 1;
        let start = start_sblock * Parameters::SUPERBLOCK_SIZE;
        let in_superblock = i - self.value_count_before_sblock(start_sblock, value);

        // Manually search for the correct block in the superblock where our match is.
        // The blocks should already be in the cache so this should be fast.
        let mut b = 0;
        while b < (Self::blocks_per_superblock() - 1) {
            let up_to_block = self.value_count_before_block(start_sblock, b+1, value);
            if up_to_block >= in_superblock {
                break;
            }
            b += 1;
        }

        // Final step: manually search for the fitting bit inside the target block.
        return self.bits.find_nth_x(start + b * Parameters::BLOCK_SIZE,
            in_superblock - self.value_count_before_block(start_sblock, b, value), value);
    }
}

impl<Parameters: RASBVecParameters> RankSelectVector for FastRASBVec<Parameters> where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {

    fn new(bits: BitVector) -> Self {
        let (rank, count1) = Self::init_rank(&bits);
        let count0 = bits.size() - count1;
        FastRASBVec::<Parameters> {
            bits,
            rank,
            count1,
            count0,
            pd: std::marker::PhantomData,
        }
    }

    fn select1(&self, i: usize) -> Option<usize> {
        self.generic_select(i, 1)
    }

    fn select0(&self, i: usize) -> Option<usize> {
        self.generic_select(i, 0)
    }

    fn rank(&self, i: usize) -> usize {
        self.generic_rank(i, 1)
    }

    fn access(&self, i: usize) -> u32 {
        self.bits.access(i)
    }
}

impl<Parameters: RASBVecParameters> DynamicUsage for FastRASBVec<Parameters> where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {
    fn dynamic_usage(&self) -> usize {
        self.rank.superblocks.dynamic_usage()
    }

    fn dynamic_usage_bounds(&self) -> (usize, Option<usize>) {
        (self.dynamic_usage(), Some(self.dynamic_usage()))
    }
}

pub struct SmallRASB;

impl RASBVecParameters for SmallRASB {
    const BLOCK_SIZE: usize = 4;
    const SUPERBLOCK_SIZE: usize = 8;
}

pub struct BigRASB;
impl RASBVecParameters for BigRASB {
    const BLOCK_SIZE: usize = 256;
    const SUPERBLOCK_SIZE: usize = 1024;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tst::*;

    #[test]
    fn rank_simple() {
        let bits = "1111111111111111111111";
        let rasb = FastRASBVec::<SmallRASB>::new(BitVector::new_from_string(bits));

        rasb.debug_print();

        for i in 0..bits.len() {
            println!("{} {}", i, rasb.rank(i));
            assert_eq!(rasb.rank(i), i);
        }
    }

    #[test]
    fn select_simple() {
        test_simple_select::<FastRASBVec<SmallRASB>>();
    }

    fn test_generic<Parameters: RASBVecParameters>(size: usize, nr_queries: usize, seed: u64) where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {
        let bits = generate_random_bits_string(size, seed, 0.5);
        println!("{}", bits);
        let rasb = FastRASBVec::<Parameters>::new(BitVector::new_from_string(bits.as_str()));
        rasb.debug_print();

        let slowb = BitVector::new_from_string(bits.as_str());
        let queries = generate_random_queries(nr_queries, 1, size);
        //for q in &queries {
        //    println!("{:?}", q);
        //}

        let answers_fast = queries.iter().exec_queries(&rasb);
        let answers_slow = queries.iter().exec_queries(&slowb);

        for ((a, b), q) in answers_fast.zip(answers_slow).zip(queries.iter()) {
            assert_eq!(a, b, "got {}, expected {} for query {:?}", a, b, q);
        }
    }

    #[test]
    fn test_small() {
        test_generic::<SmallRASB>(35, 30, 1);
    }

    #[test]
    fn test_big() {
        let n = BigRASB::SUPERBLOCK_SIZE * 4 + 3 * BigRASB::BLOCK_SIZE - 1;
        let q = n * 2;
        test_generic::<BigRASB>(n, q, 3);
    }

    #[test]
    fn sample_1() {
        test_sample::<FastRASBVec<BigRASB>>();
    }

    #[test]
    fn sample_2() {
        test_sample::<FastRASBVec<SmallRASB>>();
    }
}

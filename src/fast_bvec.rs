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
        CACHELINE_SIZE
    }

    fn dynamic_usage_bounds(&self) -> (usize, Option<usize>) {
        (self.dynamic_usage(), None)
    }
}

struct RankSupport<const CACHELINE_SIZE: usize> where [u16; num_blocks!(CACHELINE_SIZE)]: Sized {
    superblocks: Vec<RankSuperblock<CACHELINE_SIZE>>,
}

struct SelectSupport {
    blocks: Vec<usize>,
    total_count: usize,
}

pub trait RASBVecParameters {
    const BLOCK_SIZE: usize;
    const SUPERBLOCK_SIZE: usize;
    const SELECT_SUPERBLOCK: usize;
    const SELECT_BRUTEFORCE: usize = 2;
    const CACHELINE_SIZE: usize = ((Self::SUPERBLOCK_SIZE / Self::BLOCK_SIZE) * 2 + 8);
}

pub struct FastRASBVec<Parameters: RASBVecParameters> where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {
    bits: BitVector,
    rank: RankSupport<{Parameters::CACHELINE_SIZE}>,
    select0: SelectSupport,
    select1: SelectSupport,
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
            select0: SelectSupport {
                blocks: vec![0],
                total_count: 0,
            },
            select1: SelectSupport {
                blocks: vec![0],
                total_count: 0,
            },
            pd: std::marker::PhantomData,
        }
    }

    pub fn initialize_for(&mut self, bits: BitVector) {
        self.bits = bits;
        self.rank = Self::init_rank(&self.bits);
        (self.select0, self.select1) = Self::init_select(&self.bits);
    }

    pub fn debug_print(&self) {
        println!("Superblocks: {:?}", self.rank.superblocks);
        println!("select0: {:?}", self.select0.blocks);
        println!("select1: {:?}", self.select1.blocks);
    }

    pub fn blocks_per_superblock() -> usize {
        Parameters::SUPERBLOCK_SIZE / Parameters::BLOCK_SIZE
    }

    fn init_rank(bits: &BitVector) -> RankSupport<{Parameters::CACHELINE_SIZE}> {
        let n_super = bits.size().div_ceil(Parameters::SUPERBLOCK_SIZE);

        let mut rk = RankSupport {
            superblocks: vec![RankSuperblock::new(); n_super],
        };

        let mut total_count: usize = 0;
        for i in 0..n_super {
            let mut sblock_count: u16 = 0;
            assert!(usize::from(u16::MAX) >= Parameters::SUPERBLOCK_SIZE);
            assert!(Self::blocks_per_superblock() <= rk.superblocks[i].blocks.len(), "Try increasing cache line size!");

            rk.superblocks[i].before = total_count;
            for j in 0..Self::blocks_per_superblock() {
                rk.superblocks[i].blocks[j] = sblock_count;
                for k in 0..Parameters::BLOCK_SIZE {
                    let bit = i * Parameters::SUPERBLOCK_SIZE + j * Parameters::BLOCK_SIZE + k;
                    if bit < bits.size() {
                        sblock_count += bits.access(bit) as u16;
                    }
                }
            }

            total_count += sblock_count as usize;
        }

        rk
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

    fn init_select(bits: &BitVector) -> (SelectSupport, SelectSupport) {
        let mut b0 = vec![0];
        let mut b1 = vec![0];

        let sblocks = [&mut b0, &mut b1];
        let mut count_in_last_sblock = [0, 0];
        let mut total_count = [0, 0];

        for i in 0..bits.size() {
            let b = if bits.access(i) == 0 { 0 } else { 1 };
            if count_in_last_sblock[b] >= Parameters::SELECT_SUPERBLOCK {
                sblocks[b].push(i);
                count_in_last_sblock[b] = 0;
            }

            count_in_last_sblock[b] += 1;
            total_count[b] += 1
        }

        (
            SelectSupport {
                blocks: b0,
                total_count: total_count[0],
            },
            SelectSupport {
                blocks: b1,
                total_count: total_count[1],
            }
        )
    }

    fn generic_select(&self, i: usize, value: u32) -> Option<usize> {
        if i == 0 {
            return None;
        }

        let accel = if value == 0 { &self.select0 } else { &self.select1 };
        if i > accel.total_count {
            return None
        }

        let block = (i-1).div_floor(Parameters::SELECT_SUPERBLOCK);
        assert!(block < accel.blocks.len());
        let mut start = accel.blocks[block];
        let mut end = if block + 1 < accel.blocks.len() {
            accel.blocks[block + 1]
        } else {
            self.bits.size()
        };

        let mut start_rk = None;
        while end - start > Parameters::SELECT_BRUTEFORCE {
            let mid = (start + end) / 2;
            let rk = self.generic_rank(mid, value);
            if rk < i {
                start = mid;
                start_rk = Some(rk);
            } else {
                end = mid;
            }
        }

        let olds = start_rk.unwrap_or(self.generic_rank(start, value));
        return self.bits.find_nth_x(start, i - olds, value);
    }
}

impl<Parameters: RASBVecParameters> RankSelectVector for FastRASBVec<Parameters> where [u16; num_blocks!(Parameters::CACHELINE_SIZE)]: Sized {

    fn new(bits: BitVector) -> Self {
        let rank = Self::init_rank(&bits);
        let (select0, select1) = Self::init_select(&bits);
        FastRASBVec::<Parameters> {
            bits,
            rank,
            select0,
            select1,
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
        self.bits.dynamic_usage() +
            self.rank.superblocks.dynamic_usage() +
            self.select0.blocks.dynamic_usage() + self.select1.blocks.dynamic_usage()
    }

    fn dynamic_usage_bounds(&self) -> (usize, Option<usize>) {
        (self.dynamic_usage(), Some(self.dynamic_usage()))
    }
}

pub struct SmallRASB;

impl RASBVecParameters for SmallRASB {
    const BLOCK_SIZE: usize = 4;
    const SUPERBLOCK_SIZE: usize = 8;
    const SELECT_SUPERBLOCK: usize = 8;
}

pub struct BigRASB;
impl RASBVecParameters for BigRASB {
    const BLOCK_SIZE: usize = 256;
    const SUPERBLOCK_SIZE: usize = 1024;
    const SELECT_SUPERBLOCK: usize = 1024;
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

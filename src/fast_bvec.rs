use num::Integer;

use crate::bvec::*;

struct RankSupport {
    blocks: Vec<u16>,
    superb: Vec<usize>,
}

pub trait RASBVecParameters {
    const BLOCK_SIZE: usize;
    const SUPERBLOCK_SIZE: usize;
    const SELECT_SUPERBLOCK: usize;
}

pub struct FastRASBVec<Parameters: RASBVecParameters> {
    bits: BitVector,
    rank: RankSupport,
    select0: Vec<usize>,
    select1: Vec<usize>,
    pd: std::marker::PhantomData<Parameters>,
}

#[allow(dead_code)]
impl<Parameters: RASBVecParameters> FastRASBVec<Parameters> {
    pub fn size(&self) -> usize {
        self.bits.size()
    }

    pub fn debug_print(&self) {
        println!("Superblocks: {:?}", self.rank.superb);
        println!("Blocks: {:?}", self.rank.blocks);
    }

    pub fn blocks_per_superblock() -> usize {
        Parameters::SUPERBLOCK_SIZE / Parameters::BLOCK_SIZE
    }

    fn init_rank(bits: &BitVector) -> RankSupport {
        let n_blocks = bits.size().div_ceil(Parameters::BLOCK_SIZE);
        let n_super = bits.size().div_ceil(Parameters::SUPERBLOCK_SIZE);

        let mut superblocks: Vec<usize> = vec![];
        let mut blocks: Vec<u16> = vec![];

        let mut total_count: usize = 0;
        for i in 0..n_super {
            let mut sblock_count: u16 = 0;
            assert!(usize::from(u16::MAX) >= Parameters::SUPERBLOCK_SIZE);

            for j in 0..Self::blocks_per_superblock() {
                for k in 0..Parameters::BLOCK_SIZE {
                    let bit = i * Parameters::SUPERBLOCK_SIZE + j * Parameters::BLOCK_SIZE + k;
                    if bit < bits.size() {
                        sblock_count += bits.access(bit) as u16;
                    }
                }

                if i * Self::blocks_per_superblock() + j < n_blocks {
                    blocks.push(sblock_count as u16);
                }
            }

            total_count += sblock_count as usize;
            superblocks.push(total_count);
        }

        RankSupport {
            blocks,
            superb: superblocks,
        }
    }

    fn init_select(bits: &BitVector, value: usize) -> Vec<usize> {
        unimplemented!()
    }
}

impl<Parameters: RASBVecParameters> RankSelectVector for FastRASBVec<Parameters> {
    fn new(bits: BitVector) -> Self {
        let rank = Self::init_rank(&bits);
        let select0 = Self::init_select(&bits, 0);
        let select1 = Self::init_select(&bits, 1);
        FastRASBVec::<Parameters> {
            bits,
            rank,
            select0,
            select1,
            pd: std::marker::PhantomData,
        }
    }

    fn select1(&self, i: usize) -> Option<usize> {
        self.bits.select1(i)
    }

    fn select0(&self, i: usize) -> Option<usize> {
        self.bits.select0(i)
    }

    fn rank(&self, i: usize) -> usize {
        let (super_idx, super_rem) = i.div_rem(&Parameters::SUPERBLOCK_SIZE);
        let (block_idx, block_rem) = i.div_rem(&Parameters::BLOCK_SIZE);

        println!("super_idx: {}, super_rem: {}, block_idx: {}, block_rem: {}", super_idx, super_rem, block_idx, block_rem);

        let mut r = 0;
        if super_idx > 0 {
            r += self.rank.superb[super_idx - 1];
            println!("from superblock {}", r);
            if super_rem == 0 {
                return r;
            }
        }

        if block_idx > super_idx * Self::blocks_per_superblock() {
            r += self.rank.blocks[block_idx - 1] as usize;
            println!("from block {}", r);
        }

        for j in 1..=block_rem {
            r += self.bits.access(i - j) as usize;
        }

        r
    }

    fn access(&self, i: usize) -> u32 {
        self.bits.access(i)
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
        let bits = "1111111111011111111110011111111110";
        let rasb = FastRASBVec::<SmallRASB>::new(BitVector::new_from_string(bits));

        let mut count0 = 0;
        let mut count1 = 0;

        for i in 0..bits.len() {
            if rasb.access(i) == 0 {
                count0 += 1;
                assert!(rasb.select0(count0) == Some(i), "select0({}) = {:?}", count0, rasb.select0(count0));
            } else {
                count1 += 1;
                assert!(rasb.select1(count1) == Some(i), "select1({}) = {:?}", count1, rasb.select1(count1));
            }
        }
    }

    fn test_generic<Parameters: RASBVecParameters>(size: usize, nr_queries: usize, seed: u64) {
        let bits = generate_random_bits_string(size, seed);
        println!("{}", bits);
        let rasb = FastRASBVec::<Parameters>::new(BitVector::new_from_string(bits.as_str()));
        rasb.debug_print();

        let slowb = BitVector::new_from_string(bits.as_str());
        let queries = generate_random_queries(nr_queries, 1, size);
        for q in &queries {
            println!("{:?}", q);
        }

        let answers_fast = queries.iter().exec_queries(&rasb);
        let answers_slow = queries.iter().exec_queries(&slowb);

        for (a, b) in answers_fast.zip(answers_slow) {
            println!("{} {}", a, b);
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_small() {
        test_generic::<SmallRASB>(35, 30, 0);
    }

    #[test]
    fn test_big() {
        let n = BigRASB::SUPERBLOCK_SIZE * 4 + 3 * BigRASB::BLOCK_SIZE - 1;
        let q = n * 2;
        test_generic::<BigRASB>(n, q, 2);
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

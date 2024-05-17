use num::Integer;

use crate::bvec::*;

struct RankSupport {
    blocks: Vec<u16>,
    superb: Vec<usize>,
}

struct SelectSupport {
}

pub struct FastRASBVec<const BLOCK_SIZE: usize, const SUPERBLOCK_SIZE: usize> {
    bits: BitVector,
    rank: RankSupport,
    select0: SelectSupport,
    select1: SelectSupport,
}

#[allow(dead_code)]
impl<const BLOCK_SIZE: usize, const SUPERBLOCK_SIZE: usize> FastRASBVec<BLOCK_SIZE, SUPERBLOCK_SIZE> {
    pub fn size(&self) -> usize {
        self.bits.size()
    }

    pub fn debug_print(&self) {
        println!("Superblocks: {:?}", self.rank.superb);
        println!("Blocks: {:?}", self.rank.blocks);
    }

    pub fn blocks_per_superblock() -> usize {
        SUPERBLOCK_SIZE / BLOCK_SIZE
    }

    fn init_rank(bits: &BitVector) -> RankSupport {
        let n_blocks = bits.size().div_ceil(BLOCK_SIZE);
        let n_super = bits.size().div_ceil(SUPERBLOCK_SIZE);

        let mut superblocks: Vec<usize> = vec![];
        let mut blocks: Vec<u16> = vec![];

        let mut total_count: usize = 0;
        for i in 0..n_super {
            let mut sblock_count: u16 = 0;
            assert!(usize::from(u16::MAX) >= SUPERBLOCK_SIZE);

            for j in 0..Self::blocks_per_superblock() {
                for k in 0..BLOCK_SIZE {
                    let bit = i * SUPERBLOCK_SIZE + j * BLOCK_SIZE + k;
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

    fn init_select(bits: &BitVector, value: usize) -> SelectSupport {
        unimplemented!()
    }
}

impl<const BLOCK_SIZE: usize, const SUPERBLOCK_SIZE: usize> RankSelectVector for FastRASBVec<BLOCK_SIZE, SUPERBLOCK_SIZE> {

    fn new(bits: BitVector) -> Self {
        let rank = Self::init_rank(&bits);
        let select0 = Self::init_select(&bits, 0);
        let select1 = Self::init_select(&bits, 1);
        FastRASBVec {
            bits,
            rank,
            select0,
            select1,
        }
    }

    fn select1(&self, i: usize) -> Option<usize> {
        self.bits.select1(i)
    }

    fn select0(&self, i: usize) -> Option<usize> {
        self.bits.select0(i)
    }

    fn rank(&self, i: usize) -> usize {
        let (super_idx, super_rem) = i.div_rem(&SUPERBLOCK_SIZE);
        let (block_idx, block_rem) = i.div_rem(&BLOCK_SIZE);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tst::*;

    #[test]
    fn rank_simple() {
        let bits = "1111111111111111111111";
        let rasb: FastRASBVec<4, 8> = FastRASBVec::new(BitVector::new_from_string(bits));

        rasb.debug_print();

        for i in 0..bits.len() {
            println!("{} {}", i, rasb.rank(i));
            assert_eq!(rasb.rank(i), i);
        }
    }

    #[test]
    fn select_simple() {
        let bits = "1111111111011111111110011111111110";
        let rasb: FastRASBVec<4, 8> = FastRASBVec::new(BitVector::new_from_string(bits));

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

    fn test_generic<const BLOCK_SIZE: usize, const SUPERBLOCK_SIZE: usize>(size: usize, nr_queries: usize, seed: u64) {
        let bits = generate_random_bits_string(size, seed);
        println!("{}", bits);
        let rasb = FastRASBVec::<BLOCK_SIZE, SUPERBLOCK_SIZE>::new(BitVector::new_from_string(bits.as_str()));
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
        test_generic::<4, 8>(35, 30, 0);
    }

    #[test]
    fn test_big() {
        const BLOCK_SIZE: usize = 256;
        const SUPERBLOCK_SIZE: usize = 4096;
        let n = SUPERBLOCK_SIZE * 4 + 3 * BLOCK_SIZE - 1;
        let q = n * 2;
        test_generic::<BLOCK_SIZE, SUPERBLOCK_SIZE>(n, q, 2);
    }

    #[test]
    fn sample_1() {
        test_sample::<FastRASBVec<256, 4096>>();
    }

    #[test]
    fn sample_2() {
        test_sample::<FastRASBVec<4, 8>>();
    }
}

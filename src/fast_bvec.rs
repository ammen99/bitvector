use num::Integer;

use crate::bvec::*;
use derivative::Derivative;

type Superblock = usize;
type Block = u32;
type CacheBlock = u8;
const CACHE_BLOCK_BITS: usize = std::mem::size_of::<CacheBlock>() * 8;

pub trait RASBVecParameters {
    const BLOCK_SIZE: usize;
    const SUPERBLOCK_SIZE: usize;
    const MEGABLOCK_FACTOR: usize = 24;

    const SUPERBLOCK_BITS: usize = std::mem::size_of::<usize>() * 8;
    const BLOCK_BITS: usize = (64 - Self::SUPERBLOCK_SIZE.leading_zeros()) as usize;
    const CACHELINE_SIZE: usize = (Self::SUPERBLOCK_BITS + (Self::SUPERBLOCK_SIZE / Self::BLOCK_SIZE) * Self::BLOCK_BITS).div_ceil(8);
}

#[derive(Derivative)]
#[derivative(Clone(bound=""), Debug)]
struct RankSuperblock<Parameters: RASBVecParameters> where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
    data: [CacheBlock; Parameters::CACHELINE_SIZE],
}

impl<Parameters: RASBVecParameters> RankSuperblock<Parameters> where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
    fn new() -> Self {
        RankSuperblock::<Parameters> {
            data: [0; Parameters::CACHELINE_SIZE],
        }
    }

    fn mask_n_bits(x: usize) -> CacheBlock {
        if x == 0 {
            return 0;
        }
        CacheBlock::MAX >> (CACHE_BLOCK_BITS - x)
    }

    fn extract_bits(&self, l: usize, r: usize) -> u64 {
        let (lb, shiftl) = l.div_rem(&CACHE_BLOCK_BITS);
        let (rb, shiftr) = r.div_rem(&CACHE_BLOCK_BITS);

        if lb == rb {
            return ((self.data[lb] & Self::mask_n_bits(shiftr)) >> shiftl) as u64;
        }

        let mut result: u64 = (self.data[lb] >> shiftl) as u64;
        let mut shift: usize = CACHE_BLOCK_BITS - shiftl;

        for i in (lb + 1)..rb {
            result |= (self.data[i] as u64) << shift;
            shift += CACHE_BLOCK_BITS;
        }

        if shiftr > 0 {
            result |= ((self.data[rb] & Self::mask_n_bits(shiftr)) as u64) << shift;
        }

        result
    }

    fn write_bits(&mut self, l: usize, r: usize, mut value: u64) {
        let (lb, shiftl) = l.div_rem(&CACHE_BLOCK_BITS);
        let (rb, shiftr) = r.div_rem(&CACHE_BLOCK_BITS);

        self.data[lb] |= (value << shiftl) as CacheBlock;
        if lb == rb {
            return;
        }

        value >>= CACHE_BLOCK_BITS - shiftl;
        for i in lb + 1..rb {
            self.data[i] = (value & Self::mask_n_bits(CACHE_BLOCK_BITS) as u64) as CacheBlock;
            value >>= CACHE_BLOCK_BITS;
        }

        if shiftr > 0 {
            self.data[rb] |= value as CacheBlock;
        }
    }

    fn superblock(&self) -> Superblock {
        self.extract_bits(0, Parameters::SUPERBLOCK_BITS) as usize
    }

    fn set_super(&mut self, value: Superblock) {
        self.write_bits(0, Parameters::SUPERBLOCK_BITS, value as u64);
    }

    fn block(&self, i: usize) -> Block {
        let start = Parameters::SUPERBLOCK_BITS + i * Parameters::BLOCK_BITS;
        self.extract_bits(start, start + Parameters::BLOCK_BITS) as Block
    }

    fn set_block(&mut self, i: usize, value: Block) {
        let start = Parameters::SUPERBLOCK_BITS + i * Parameters::BLOCK_BITS;
        self.write_bits(start, start + Parameters::BLOCK_BITS, value as u64);
    }
}

struct RankSupport<Parameters: RASBVecParameters> where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
    superblocks: Vec<RankSuperblock<Parameters>>,
}

pub struct FastRASBVec<Parameters: RASBVecParameters> where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
    bits: BitVector,
    rank: RankSupport<Parameters>,
    megablocks: Vec<usize>,
    count0: usize,
    count1: usize,
    pd: std::marker::PhantomData<Parameters>,
}

#[allow(dead_code)]
impl<Parameters: RASBVecParameters> FastRASBVec<Parameters> where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
    pub fn size(&self) -> usize {
        self.bits.size()
    }

    pub fn new_empty() -> Self {
        FastRASBVec::<Parameters> {
            bits: BitVector::new_from_string("0"),
            rank: RankSupport {
                superblocks: vec![],
            },
            megablocks: vec![],
            count0: 0,
            count1: 0,
            pd: std::marker::PhantomData,
        }
    }

    pub fn initialize_for(&mut self, bits: BitVector) {
        self.init_rank(&bits);
        self.bits = bits;
    }

    pub fn blocks_per_superblock() -> usize {
        Parameters::SUPERBLOCK_SIZE / Parameters::BLOCK_SIZE
    }

    fn init_rank(&mut self, bits: &BitVector) {
        let n_super = bits.size().div_ceil(Parameters::SUPERBLOCK_SIZE);

        let mut rk = RankSupport {
            superblocks: vec![RankSuperblock::new(); n_super],
        };

        let mut megablocks = vec![];
        megablocks.reserve(n_super.div_floor(Parameters::MEGABLOCK_FACTOR));

        let mut total_count: Superblock = 0;
        for i in 0..n_super {
            let mut sblock_count: Block = 0;
            assert!(Block::MAX as usize >= (Self::blocks_per_superblock() - 1) * Parameters::BLOCK_SIZE,
                "Superblock size is too big for block max type.");

            if i % Parameters::MEGABLOCK_FACTOR == 0 {
                megablocks.push(total_count);
            }

            rk.superblocks[i].set_super(total_count);
            for j in 0..Self::blocks_per_superblock() {
                rk.superblocks[i].set_block(j, sblock_count);

                let block_start = i * Parameters::SUPERBLOCK_SIZE + j * Parameters::BLOCK_SIZE;
                if block_start < bits.size() {
                    let block_end = std::cmp::min(block_start + Parameters::BLOCK_SIZE, bits.size());
                    sblock_count += bits.count_ones(block_start, block_end) as Block;
                }
            }

            total_count += sblock_count as usize;
        }

        self.count1 = total_count;
        self.count0 = bits.size() - total_count;
        self.rank = rk;
        self.megablocks = megablocks;
    }

    fn _rank1(&self, i: usize) -> usize {
        let (super_idx, super_rem) = i.div_rem(&Parameters::SUPERBLOCK_SIZE);
        let (block_idx, block_rem) = super_rem.div_rem(&Parameters::BLOCK_SIZE);

        //println!("super_idx: {}, super_rem: {}, block_idx: {}, block_rem: {}", super_idx, super_rem, block_idx, block_rem);

        let mut r = self.rank.superblocks[super_idx].superblock();
        r += self.rank.superblocks[super_idx].block(block_idx) as usize;
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
            self.rank.superblocks[b].superblock()
        } else {
            b * Parameters::SUPERBLOCK_SIZE - self.rank.superblocks[b].superblock()
        }
    }

    fn value_count_before_block(&self, sb: usize, b: usize, value: u32) -> usize {
        if value == 1 {
            self.rank.superblocks[sb].block(b) as usize
        } else {
            b * Parameters::BLOCK_SIZE - self.rank.superblocks[sb].block(b) as usize
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

        // Step 1: binary search over megablocks
        let mut mega_l = 0usize;
        let mut mega_r = self.megablocks.len();
        while mega_r - mega_l > 1 {
            let mid = (mega_l + mega_r) / 2;
            let before = if value == 0 {
                    mid * Parameters::MEGABLOCK_FACTOR * Parameters::SUPERBLOCK_SIZE - self.megablocks[mid]
                }
                else {
                    self.megablocks[mid]
                };

            if before >= i {
                mega_r = mid;
            } else {
                mega_l = mid;
            }
        }

        let lsblock = mega_l * Parameters::MEGABLOCK_FACTOR;
        let mut rsblock = std::cmp::min(lsblock + Parameters::MEGABLOCK_FACTOR, self.rank.superblocks.len());

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

impl<Parameters: RASBVecParameters> RankSelectVector for FastRASBVec<Parameters> where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
    fn new(bits: BitVector) -> Self {
        let mut vec = Self::new_empty();
        vec.initialize_for(bits);
        vec
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

    fn get_memory_usage(&self) -> usize {
        self.megablocks.len() * std::mem::size_of::<usize>() +
            self.rank.superblocks.len() * std::mem::size_of::<RankSuperblock<Parameters>>()
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

struct RankBitTestParams;
impl RASBVecParameters for RankBitTestParams {
    const BLOCK_SIZE: usize = 4;
    const SUPERBLOCK_SIZE: usize = 8;

    const SUPERBLOCK_BITS: usize = 19;
    const BLOCK_BITS: usize = 13;
    const CACHELINE_SIZE: usize = (Self::SUPERBLOCK_BITS + 55 * Self::BLOCK_BITS).div_ceil(CACHE_BLOCK_BITS);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tst::*;
    use rand::{Rng, SeedableRng};
    use rand_xoshiro::Xoshiro256Plus;

    #[test]
    fn rank_superblock_test() {
        for test in 0..10 {
            let mut bfield = RankSuperblock::<RankBitTestParams>::new();
            let mut rng = Xoshiro256Plus::seed_from_u64(233 * test);

            let sblock = rng.gen_range(0..(1 << RankBitTestParams::SUPERBLOCK_BITS));
            bfield.set_super(sblock);
            let mut blocks = [0; 10];
            for i in 0..10 {
                let block = rng.gen_range(0..(1 << RankBitTestParams::BLOCK_BITS));
                bfield.set_block(i, block);
                blocks[i] = block;
            }

            assert_eq!(bfield.superblock(), sblock, "superblock not ok");
            for i in 0..10 {
                assert_eq!(bfield.block(i), blocks[i], "block {} not ok", i);
            }
        }
    }

    #[test]
    fn rank_simple() {
        let bits = "1111111111111111111111";
        let rasb = FastRASBVec::<SmallRASB>::new(BitVector::new_from_string(bits));

        for i in 0..bits.len() {
            println!("{} {}", i, rasb.rank(i));
            assert_eq!(rasb.rank(i), i);
        }
    }

    #[test]
    fn select_simple() {
        test_simple_select::<FastRASBVec<SmallRASB>>();
    }

    fn test_generic<Parameters: RASBVecParameters>(size: usize, nr_queries: usize, seed: u64) where [CacheBlock; Parameters::CACHELINE_SIZE]: Sized {
        let bits = generate_random_bits_string(size, seed, 0.5);
        println!("{}", bits);
        let rasb = FastRASBVec::<Parameters>::new(BitVector::new_from_string(bits.as_str()));
        let slowb = BitVector::new_from_string(bits.as_str());
        let queries = generate_random_queries(nr_queries, 1, size, None);
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

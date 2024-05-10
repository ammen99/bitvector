use crate::bvec::*;

const SUPERBLOCK_SIZE: usize = 4096;
const BLOCK_SIZE: usize = 256;

pub struct FastRASBVec {
    bits: BitVector,
    blocks: Vec<u32>,
    superblocks: Vec<u32>,
}

impl FastRASBVec {
    pub fn size(&self) -> usize {
        self.bits.size()
    }
}

impl RankSelectVector for FastRASBVec {
    fn new(bits: BitVector) -> Self {
        let n_blocks = bits.size().div_ceil(BLOCK_SIZE);
        let n_super = bits.size().div_ceil(SUPERBLOCK_SIZE);

        FastRASBVec {
            bits,
            blocks: vec![0; n_blocks],
            superblocks: vec![0; n_super],
        }
    }

    fn select1(&self, i: usize) -> Option<usize> {
        self.bits.select1(i)
    }

    fn select0(&self, i: usize) -> Option<usize> {
        self.bits.select0(i)
    }

    fn rank(&self, i: usize) -> usize {
        self.bits.rank(i)
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
    fn test() {
        let size = SUPERBLOCK_SIZE * 10 + 10;
        let nr_queries = 4 * size;
        let bits = generate_random_bits_string(size, 0);
        let rasb = FastRASBVec::new(BitVector::new_from_string(bits.as_str()));
        let slowb = BitVector::new_from_string(bits.as_str());
        let queries = generate_random_queries(nr_queries, 1, size);

        let answers_fast = queries.iter().exec_queries(&rasb);
        let answers_slow = queries.iter().exec_queries(&slowb);

        for (a, b) in answers_fast.zip(answers_slow) {
            assert!(a == b);
        }
    }
}

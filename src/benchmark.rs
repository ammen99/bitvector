use crate::fast_bvec::*;

struct Params<const A: usize, const B: usize, const C: usize>;

impl<const A: usize, const B: usize, const C: usize> RASBVecParameters for Params<A, B, C> {
    const BLOCK_SIZE: usize = A;
    const SUPERBLOCK_SIZE: usize = B;
    const SELECT_SUPERBLOCK: usize = C;
}

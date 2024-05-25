use std::mem::size_of;

type BitCell = u128;
const BIT_CELL_SIZE: usize = size_of::<BitCell>();

pub struct BitVector {
    bits: Vec<BitCell>,
    size: usize,
}

impl BitVector {
    pub fn new(bits: Vec<bool>) -> Self {
        let mut v = vec![0; (bits.len() + BIT_CELL_SIZE -1 ) / BIT_CELL_SIZE];
        for i in 0..bits.len() {
            if bits[i] {
                v[i / BIT_CELL_SIZE] |= (1 as BitCell) << (i % BIT_CELL_SIZE);
            }
        }
        BitVector {
            bits: v,
            size: bits.len(),
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn new_from_string(bits: &str) -> Self {
        return BitVector::new(bits.chars().map(|x| x == '1').collect())
    }

    // Get the i'th element of the bitvector
    pub fn get_nth(&self, i: usize) -> u32 {
        assert!(i < self.size);
        return ((self.bits[i / BIT_CELL_SIZE] >> (i % BIT_CELL_SIZE)) & 1) as u32;
    }

    fn select_x(&self, mut i: usize, x: u32) -> Option<usize> {
        if i == 0 {
            return None;
        }

        for j in 0..self.size() {
            if self.get_nth(j) == x {
                i -= 1;
                if i == 0 {
                    return Some(j);
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tst;

    #[test]
    fn test() {
        let str = tst::generate_random_bits_string(3*128 + 15, 0, 0.5);

        let bv = BitVector::new_from_string(str.as_str());
        assert_eq!(bv.size(), str.len());

        for i in 0..str.len() {
            assert_eq!(bv.get_nth(i), str.chars().nth(i).unwrap() as u32 - '0' as u32);
        }
    }
}

pub trait RankSelectVector {
    #[allow(dead_code)]
    fn new(bits: BitVector) -> Self;

    // Get the position of the i'th 1 in the bit vector
    fn select1(&self, i: usize) -> Option<usize>;

    // Get the position of the i'th 0 in the bit vector
    fn select0(&self, i: usize) -> Option<usize>;

    // Return the number of 1s in the bit vector on positions [0, ... i).
    fn rank(&self, i: usize) -> usize;

    // Return the value of the ith bit
    fn access(&self, i: usize) -> u32;
}

impl RankSelectVector for BitVector {
    fn new(bits: BitVector) -> Self {
        bits
    }

    fn select1(&self, i: usize) -> Option<usize> {
        self.select_x(i, 1)
    }

    fn select0(&self, i: usize) -> Option<usize> {
        self.select_x(i, 0)
    }

    fn rank(&self, i: usize) -> usize {
        let mut count = 0;
        for j in 0..i {
            if self.get_nth(j) == 1 {
                count += 1;
            }
        }
        count
    }

    fn access(&self, i: usize) -> u32 {
        self.get_nth(i)
    }
}

#[cfg(test)]
mod rank_select_naive_test {
    use super::*;
    use crate::tst;

    #[test]
    fn test() {
        tst::test_sample::<BitVector>();
    }

    #[test]
    fn select() {
        tst::test_simple_select::<BitVector>();
    }
}

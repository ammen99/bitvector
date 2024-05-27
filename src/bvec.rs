use std::mem::size_of;
use memuse::DynamicUsage;
use num::Integer;

type BitCell = u64;
const BIT_CELL_SIZE: usize = size_of::<BitCell>() * 8;

pub struct BitVector {
    bits: Vec<BitCell>,
    size: usize,
}

impl BitVector {
    pub fn new(bits: Vec<bool>) -> Self {
        let mut v = vec![0; bits.len().div_ceil(BIT_CELL_SIZE)];
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

    fn count_ones_block(&self, b: usize, l: usize, r: usize) -> usize {
        let mut v = self.bits[b];
        if r < BIT_CELL_SIZE {
            v &= ((1 as BitCell) << r) - 1;
        }
        v >>= l;
        v.count_ones() as usize
    }

    // Count the number of ones in [l, r)
    pub fn count_ones(&self, l: usize, r: usize) -> usize {
        let (mut s_block, s_offset) = l.div_rem(&BIT_CELL_SIZE);
        let (e_block, e_offset) = r.div_rem(&BIT_CELL_SIZE);

        if s_block == e_block {
            return self.count_ones_block(s_block, s_offset, e_offset);
        }

        let mut count = 0;

        if s_offset != 0 {
            count += self.count_ones_block(s_block, s_offset, BIT_CELL_SIZE);
            s_block += 1;
        }

        for b in s_block..e_block {
            count += self.bits[b].count_ones() as usize;
        }

        if e_offset != 0 {
            count += self.count_ones_block(e_block, 0, e_offset);
        }

        return count
    }

    pub fn find_nth_x(&self, start: usize, mut nth: usize, x: u32) -> Option<usize> {
        if nth == 0 {
            return None;
        }

        for j in start..self.size() {
            if self.get_nth(j) == x {
                nth -= 1;
                if nth == 0 {
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

    #[test]
    fn test_count_ones() {
        let n = 3*128 + 15;
        let str = tst::generate_random_bits_string(n, 0, 0.5);
        println!("{}", str);
        let bv = BitVector::new_from_string(str.as_str());

        for i in 0..n {
            for j in (i+1)..n {
                let mut ans = 0;
                for k in i..j {
                    ans += str.as_bytes()[k] as usize - '0' as usize;
                }

                let actual = bv.count_ones(i, j);
                assert_eq!(ans, actual, "i = {}, j = {} ans = {} actual = {}", i, j, ans, actual);
            }
        }
    }

    #[test]
    fn find_nth_x() {
        let n = 3*128 + 15;
        let str = tst::generate_random_bits_string(n, 0, 0.5);
        println!("{}", str);
        let bv = BitVector::new_from_string(str.as_str());

        for i in 0..n {
            let mut count0 = 0;
            let mut count1 = 0;
            for j in i..n {
                if bv.get_nth(j) == 0 {
                    count0 += 1;
                    assert_eq!(Some(j), bv.find_nth_x(i, count0, 0), "i = {}, j = {}, count0 = {}", i, j, count0);
                } else {
                    count1 += 1;
                    assert_eq!(Some(j), bv.find_nth_x(i, count1, 1), "i = {}, j = {}, count1 = {}", i, j, count1);
                }
            }
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
        self.find_nth_x(0, i, 1)

    }

    fn select0(&self, i: usize) -> Option<usize> {
        self.find_nth_x(0, i, 0)
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

impl DynamicUsage for BitVector {
    fn dynamic_usage(&self) -> usize {
        self.bits.dynamic_usage()
    }

    fn dynamic_usage_bounds(&self) -> (usize, Option<usize>) {
        self.bits.dynamic_usage_bounds()
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

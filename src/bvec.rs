use std::mem::size_of;
use memuse::DynamicUsage;
use num::Integer;
use cfg_if::cfg_if;

use std::fs::File;
use std::io::BufReader;
use std::io::Read;
use std::io::BufRead;
use std::iter::Iterator;

type BitCell = u64;
const BIT_CELL_SIZE: usize = size_of::<BitCell>() * 8;

pub struct BitVector {
    bits: Vec<BitCell>,
    size: usize,
}

impl BitVector {
    pub fn size(&self) -> usize {
        self.size
    }

    pub fn new_from_input(file: &mut BufReader<File>) -> Self {
        let mut v = vec![];
        const BUF_SIZE: u64 = 1 << 26;
        let mut buf = vec![];
        let mut total_size: usize = 0;
        let mut idx: usize = 0;

        loop {
            let n = file.by_ref().take(BUF_SIZE).read_until('\n' as u8, &mut buf).unwrap();

            buf.reserve(n);
            for i in 0..n {
                if buf[i] == '\n' as u8 {
                    return Self {
                        bits: v,
                        size: total_size,
                    }
                }

                if idx % BIT_CELL_SIZE == 0 {
                    v.push(0);
                    idx = 0;
                }

                if buf[i] == '1' as u8 {
                    *v.last_mut().unwrap() |= (1 as BitCell) << idx;
                }

                idx += 1;
                total_size += 1;
            }

            buf.clear();
        }
    }

    pub fn new_from_string(bits: &str) -> Self {
        let mut v = vec![0; bits.len().div_ceil(BIT_CELL_SIZE)];
        let mut bytes = bits.bytes();
        for i in 0..bits.len() {
            if bytes.next().unwrap() == '1' as u8 {
                v[i / BIT_CELL_SIZE] |= (1 as BitCell) << (i % BIT_CELL_SIZE);
            }
        }
        BitVector {
            bits: v,
            size: bits.len(),
        }
    }

    // Get the i'th element of the bitvector
    pub fn get_nth(&self, i: usize) -> u32 {
        assert!(i < self.size);
        return ((self.bits[i / BIT_CELL_SIZE] >> (i % BIT_CELL_SIZE)) & 1) as u32;
    }

    fn count_ones_bit_cell(&self, b: usize, l: usize, r: usize) -> usize {
        let mut v = self.bits[b];
        if r < BIT_CELL_SIZE {
            v &= ((1 as BitCell) << r) - 1;
        }
        v >>= l;
        v.count_ones() as usize
    }

    fn count_x_in_bit_cell(&self, b: usize, l: usize, r: usize, x: u32) -> usize {
        if x == 1 {
            return self.count_ones_bit_cell(b, l, r);
        } else {
            return (r - l) - self.count_ones_bit_cell(b, l, r);
        }
    }

    // Count the number of ones in [l, r)
    pub fn count_ones(&self, l: usize, r: usize) -> usize {
        let (mut s_bit_cell, s_offset) = l.div_rem(&BIT_CELL_SIZE);
        let (e_bit_cell, e_offset) = r.div_rem(&BIT_CELL_SIZE);

        if s_bit_cell == e_bit_cell {
            return self.count_ones_bit_cell(s_bit_cell, s_offset, e_offset);
        }

        let mut count = 0;

        if s_offset != 0 {
            count += self.count_ones_bit_cell(s_bit_cell, s_offset, BIT_CELL_SIZE);
            s_bit_cell += 1;
        }

        for b in s_bit_cell..e_bit_cell {
            count += self.bits[b].count_ones() as usize;
        }

        if e_offset != 0 {
            count += self.count_ones_bit_cell(e_bit_cell, 0, e_offset);
        }

        return count
    }

    fn find_nth_set_bit_slow(&self, mut bit_cell: BitCell, mut nth: usize) -> usize {
        for i in 0..BIT_CELL_SIZE {
            nth -= (bit_cell & 1) as usize;
            if nth == 0 {
                return i;
            }
            bit_cell >>= 1;
        }
        panic!("Should not be reached!");
    }

    fn find_nth_set_bit(&self, bit_cell: BitCell, nth: usize) -> usize {
        if BIT_CELL_SIZE != 64 {
            return self.find_nth_set_bit_slow(bit_cell, nth);
        }

        cfg_if! {
            if #[cfg(target_arch = "x86_64")] {
                use core::arch::x86_64::_pdep_u64;
                let mask = (1 as BitCell) << (nth - 1);
                let r: u64 = unsafe {
                    _pdep_u64(mask, bit_cell)
                };

                r.trailing_zeros() as usize
            } else {
                self.find_nth_set_bit_slow(bit_cell, nth)
            }
        }
    }

    // Find nth x in a bit_cell with index b, starting at offset l.
    // Does not find matches beyond the end of the particular bit cell.
    fn find_nth_x_in_bit_cell(&self, b: usize, l: usize, nth: usize, x: u32) -> Option<usize> {
        if nth == 0 {
            return None;
        }

        let mut b = self.bits[b] >> l;
        if x == 0 {
            b = !b;
        }

        return Some(self.find_nth_set_bit(b, nth) + l);
    }

    pub fn find_nth_x(&self, start: usize, mut nth: usize, x: u32) -> Option<usize> {
        if nth == 0 {
            return None;
        }

        let (mut cur_bit_cell, mut cur_offset) = start.div_rem(&BIT_CELL_SIZE);

        loop {
            let in_cur_bit_cell_count = self.count_x_in_bit_cell(cur_bit_cell, cur_offset, BIT_CELL_SIZE, x);
            if nth <= in_cur_bit_cell_count {
                return self.find_nth_x_in_bit_cell(cur_bit_cell, cur_offset, nth, x)
                    .map(|x| x + cur_bit_cell * BIT_CELL_SIZE)
                    .take_if(|x| *x < self.size());
            }

            nth -= in_cur_bit_cell_count;
            cur_bit_cell += 1;
            cur_offset = 0;

            if cur_bit_cell >= self.bits.len() {
                return None;
            }
        }
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

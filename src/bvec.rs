pub struct BitVector {
    bits: Vec<u128>,
    size: usize,
}

impl BitVector {
    pub fn new(bits: Vec<bool>) -> Self {
        let mut v = vec![0; (bits.len() + 127) / 128];
        for i in 0..bits.len() {
            if bits[i] {
                v[i / 128] |= 1u128 << (i % 128);
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
    pub fn access(&self, i: usize) -> bool {
        assert!(i < self.size);
        return (self.bits[i / 128] >> (i % 128)) & 1 == 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tst;
    use seeded_random::{Random, Seed};

    #[test]
    fn test() {
        let mut rng = Random::from_seed(Seed::unsafe_new(0));
        let str = tst::generate_random_bits_string(3*128 + 15, &mut rng);

        let bv = BitVector::new_from_string(str.as_str());
        println!("{}", str);
        assert_eq!(bv.size(), str.len());

        for i in 0..str.len() {
            assert_eq!(bv.access(i), str.chars().nth(i).unwrap() == '1');
        }
    }
}

pub trait RankSelectVector {
    fn new(bits: BitVector) -> Self;

    // Get the position of the i'th 1 in the bit vector
    fn select(&self, i: usize) -> Option<usize>;

    // Return the number of 1s in the bit vector on positions [0, ... i).
    fn rank(&self, i: usize) -> usize;
}

impl RankSelectVector for BitVector {
    fn new(bits: BitVector) -> Self {
        bits
    }

    fn select(&self, mut i: usize) -> Option<usize> {
        for j in 0..self.bits.len() {
            if self.access(j) {
                if i == 0 {
                    return Some(j);
                }
                i -= 1;
            }
        }

        None
    }

    fn rank(&self, i: usize) -> usize {
        let mut count = 0;
        for j in 0..i {
            if self.access(j) {
                count += 1;
            }
        }
        count
    }
}

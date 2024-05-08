//use crate::bvec::BitVector;

pub struct NaiveBitVector {
    bits: Vec<bool>,
}

//impl BitVector for NaiveBitVector {
//    fn new(bits: Vec<bool>) -> Self {
//        NaiveBitVector {
//            bits
//        }
//    }
//
//    fn rank(&self, i: usize) -> usize {
//        let mut count = 0;
//        for j in 0..i {
//            if self.access(j) {
//                count += 1;
//            }
//        }
//        count
//    }
//    fn access(&self, i: usize) -> bool {
//        self.bits[i]
//    }
//    fn select(&self, mut i: usize) -> Option<usize> {
//        for j in 0..self.bits.len() {
//            if self.access(j) {
//                if i == 0 {
//                    return Some(j);
//                } else {
//                    i -= 1;
//                }
//            }
//        }
//
//        return None
//    }
//}
//
//#[cfg(test)]
//mod tests {
//    use super::*;
//    use bvec::BitVector;
//    use naive_bvec::NaiveBitVector;
//
//    #[test]
//    fn test_rank() {
//        let bv = NaiveBitVector::new(vec![true, false, true, false, true]);
//        assert_eq!(bv.rank(0), 0);
//        assert_eq!(bv.rank(1), 1);
//        assert_eq!(bv.rank(2), 2);
//        assert_eq!(bv.rank(3), 2);
//        assert_eq!(bv.rank(4), 4);
//    }
//
//    #[test]
//    fn test_select() {
//        let bv = NaiveBitVector::new(vec![true, false, true, false, true]);
//        assert_eq!(bv.select(0), Some(0));
//        assert_eq!(bv.select(1), Some(1));
//        assert_eq!(bv.select(2), Some(2));
//        assert_eq!(bv.select(3), Some(2));
//        assert_eq!(bv.select(4), Some(4));
//    }
//}

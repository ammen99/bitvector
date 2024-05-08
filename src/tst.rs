use crate::bvec;
use seeded_random::Random;

pub fn generate_random_bits_string(length: usize, rng: &mut Random) -> String {
    let mut result = String::with_capacity(length);
    for _ in 0..length {
        if rng.gen() {
            result.push('1');
        } else {
            result.push('0');
        }
    }

    result
}

pub fn test_rank<T: bvec::RankSelectVector>() {
    let bv = T::new(bvec::BitVector::new_from_string("0110011000010"));

    assert_eq!(bv.rank(0), 0);
    assert_eq!(bv.rank(1), 1);
    assert_eq!(bv.rank(2), 1);
    assert_eq!(bv.rank(3), 2);
    assert_eq!(bv.rank(4), 2);
}

fn test_select<T: bvec::RankSelectVector>() {
    unimplemented!()
}

fn test_sample<T: bvec::RankSelectVector>() {
    unimplemented!()
}

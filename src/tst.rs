use crate::bvec::{self};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;

pub struct SectionDescription {
    pub weight0: f32,
    pub section_len: usize,
    pub probability: f32,
}

pub fn generate_random_bits_in_sections(section_description: &[SectionDescription], nr_sections: usize, seed: u64) -> String {
    let mut rng = Xoshiro256Plus::seed_from_u64(seed);
    let mut result = String::new();

    let weight_sum = section_description.iter().map(|s| s.probability).sum::<f32>();

    for _ in 0..nr_sections {
        let mut choice = rng.gen_range(0.0..weight_sum);
        let mut section = section_description.last().unwrap();
        for s in section_description {
            if choice < s.probability {
                section = s;
                break;
            } else {
                choice -= s.probability;
            }
        }

        for _ in 0..section.section_len {
            result.push(if rng.gen_range(0.0..1.0) < section.weight0 { '0' } else {'1'});
        }
    }

    result
}

pub fn generate_random_bits_string(length: usize, seed: u64, weight0: f32) -> String {
    let desc = [SectionDescription {
        weight0, section_len: length, probability: 1.0
    }];

    generate_random_bits_in_sections(&desc, 1, seed)
}

#[derive(Debug)]
pub enum Query {
    Access(usize),
    Select1(usize),
    Select0(usize),
    Rank1(usize),
    Rank0(usize),
}

pub fn generate_random_queries(nr_queries: usize, seed: u64, n: usize, count1: Option<usize>) -> Vec<Query> {
    let full_range = 0..n;
    let range1 = 0..count1.unwrap_or(n);
    let range0 = 0..(n - count1.unwrap_or(0));

    let mut rng = Xoshiro256Plus::seed_from_u64(seed);
    (0..nr_queries).map(|_| {
        let qtype = rng.gen_range(0..5);
        match qtype {
            0 => Query::Access(rng.gen_range(full_range.clone())),
            1 => Query::Select1(rng.gen_range(range1.clone())),
            2 => Query::Select0(rng.gen_range(range0.clone())),
            3 => Query::Rank1(rng.gen_range(full_range.clone())),
            4 => Query::Rank0(rng.gen_range(full_range.clone())),
            _ => panic!()
        }
    }).collect()
}

pub trait ExecQueries {
    fn exec_queries<'a>(self, b: &'a impl bvec::RankSelectVector) -> impl Iterator<Item = usize> + 'a where Self: 'a;
}

pub fn exec_one_query(q: &Query, b: &impl bvec::RankSelectVector) -> usize {
    match q {
        Query::Access(i) => b.access(*i) as usize,
        Query::Select1(i) => b.select1(*i).unwrap_or(usize::MAX),
        Query::Select0(i) => b.select0(*i).unwrap_or(usize::MAX),
        Query::Rank1(i) => b.rank(*i),
        Query::Rank0(i) => i - b.rank(*i),
    }

}

impl<'b, I: Iterator<Item = &'b Query>> ExecQueries for I {
    fn exec_queries<'a>(self, b: &'a impl bvec::RankSelectVector) -> impl Iterator<Item = usize> + 'a where I: 'a {
        self.map(|q| {
            exec_one_query(q, b)
        })
    }
}

pub fn check_answers(b: &impl bvec::RankSelectVector, qs: &Vec<Query>, answers: &Vec<usize>) {
    let vals = qs.iter().exec_queries(b).collect::<Vec<_>>();
    assert_eq!(vals.len(), answers.len());
    for (idx, val) in vals.iter().enumerate() {
        let a = answers[idx];
        assert!(*val == a, "expected {a}, got {val} for idx={idx} q={:?}", qs[idx]);
    }
}

pub fn test_sample<T: bvec::RankSelectVector>() {
    let b = T::new(bvec::BitVector::new_from_string("001110110101010111111111"));
    let qs = vec![
        Query::Access(4),
        Query::Rank0(10),
        Query::Select1(14),
        Query::Rank1(10),
        Query::Select0(3),
        Query::Access(5),
    ];

    check_answers(&b, &qs, &vec![1, 4, 20, 6, 5, 0]);
}

pub fn test_simple_select<T: bvec::RankSelectVector>() {
    let bits = "1111111111011111111110011111111110";
    println!("{}", bits);
    let rasb = T::new(bvec::BitVector::new_from_string(bits));

    let mut count0 = 0;
    let mut count1 = 0;

    for i in 0..bits.len() {
        if rasb.access(i) == 0 {
            count0 += 1;

            let sel = rasb.select0(count0);
            assert!(sel == Some(i), "select0({}) = {:?} (should be {})", count0, sel, i);
        } else {
            count1 += 1;
            let sel = rasb.select1(count1);
            assert!(sel == Some(i), "select1({}) = {:?} (should be {})", count1, sel, i);
        }
    }
}

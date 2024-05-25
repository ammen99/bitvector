use crate::bvec;
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

pub fn generate_random_queries(nr_queries: usize, seed: u64, n: usize) -> Vec<Query> {
    let mut rng = Xoshiro256Plus::seed_from_u64(seed);
    (0..nr_queries).map(|_| {
        let qtype = rng.gen_range(0..5);
        let pos = rng.gen_range(0..n as u32) as usize;

        match qtype {
            0 => Query::Access(pos),
            1 => Query::Select1(pos),
            2 => Query::Select0(pos),
            3 => Query::Rank1(pos),
            4 => Query::Rank0(pos),
            _ => panic!()
        }
    }).collect()
}

pub trait ExecQueries {
    fn exec_queries<'a>(self, b: &'a impl bvec::RankSelectVector) -> impl Iterator<Item = usize> + 'a where Self: 'a;
}

impl<'b, I: Iterator<Item = &'b Query>> ExecQueries for I {
    fn exec_queries<'a>(self, b: &'a impl bvec::RankSelectVector) -> impl Iterator<Item = usize> + 'a where I: 'a {
        self.map(|q| {
            match q {
                Query::Access(i) => b.access(*i) as usize,
                Query::Select1(i) => b.select1(*i).unwrap_or(usize::MAX),
                Query::Select0(i) => b.select0(*i).unwrap_or(usize::MAX),
                Query::Rank1(i) => b.rank(*i),
                Query::Rank0(i) => i - b.rank(*i),
            }
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

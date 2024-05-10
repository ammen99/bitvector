use crate::bvec;
use seeded_random::{Random, Seed};

pub fn generate_random_bits_string(length: usize, seed: u64) -> String {
    let rng = Random::from_seed(Seed::unsafe_new(seed));
    let mut result = String::with_capacity(length);
    for _ in 0..length {
        result.push(if rng.gen() { '1' } else {'0'});
    }

    result
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
    let rng = Random::from_seed(Seed::unsafe_new(seed));
    (0..nr_queries).map(|_| {
        let qtype = rng.range(0, 5);
        let pos = rng.range(0, n as u32) as usize;

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

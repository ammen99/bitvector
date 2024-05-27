#![feature(impl_trait_in_assoc_type)]
#![feature(int_roundings)]
mod bvec;
mod tst;
mod fast_bvec;
mod benchmark;

use crate::benchmark::*;

fn main() {
    //benchmark_rank();
    benchmark_select_all(&[AllBench::Bruteforce]);
}

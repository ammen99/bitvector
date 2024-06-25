#![feature(impl_trait_in_assoc_type)]
#![feature(trait_alias)]
#![feature(int_roundings)]
#![feature(option_take_if)]
#![feature(generic_const_exprs)]
mod bvec;
mod tst;
mod fast_bvec;
mod benchmark;

use memuse::DynamicUsage;
use tst::Query;

use crate::benchmark::*;
use crate::tst::ExecQueries;
use crate::bvec::RankSelectVector;
use std::io::Write;

fn praktikum_main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        panic!("Usage: {} <input> <output>", args[0]);
    }

    let input = &args[1];
    let output = &args[2];

    if let Ok(bits) = std::fs::read_to_string(input) {
        let mut lines = bits.lines();
        let n = lines.next().unwrap().trim().parse::<usize>().unwrap();
        let bits = lines.next().unwrap().trim();
        let qs = lines.map(|line| {
            let mut line = line.trim().split(' ');

            let cmd = line.next().unwrap();
            let v = line.next().unwrap().parse::<usize>().unwrap();

            match cmd {
                "access" => Query::Access(v),
                "rank" => if v == 0 {
                    Query::Rank0(line.next().unwrap().parse::<usize>().unwrap())
                } else {
                    Query::Rank1(line.next().unwrap().parse::<usize>().unwrap())
                },
                "select" => if v == 0 {
                    Query::Select0(line.next().unwrap().parse::<usize>().unwrap())
                } else {
                    Query::Select1(line.next().unwrap().parse::<usize>().unwrap())
                },
                _ => panic!("Unknown query type encounted in the input file!")
            }
        }).collect::<Vec<_>>();
        assert!(qs.len() == n);

        let bv = bvec::BitVector::new_from_string(bits);
        let used_space;
        let mut answers: Vec<usize> = vec![0; n];
        let accel_bv;

        let time_build = measure_time!({
            accel_bv = fast_bvec::FastRASBVec::<Params<512, 8192, 16>>::new(bv);
        });

        let time_query = measure_time!({
            used_space = accel_bv.dynamic_usage();
            for (i, q) in qs.iter().exec_queries(&accel_bv).enumerate() {
                answers[i] = q;
            };
        });

        // Write to output, one query per line
        let mut out = std::fs::File::create(output).unwrap();
        for answer in answers {
            writeln!(out, "{}", answer).unwrap();
        }

        println!("RESULT name=Ilia_Bozhinov time_build={} time_query={} space={}", time_build, time_query, used_space*8);
        return;
    }

    panic!("Failed to read input file!");
}

fn main() {
    praktikum_main();
    //benchmark_rank();
    //benchmark_select_all(&[AllBench::Bruteforce]);
}

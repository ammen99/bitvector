#![feature(impl_trait_in_assoc_type)]
#![feature(trait_alias)]
#![feature(int_roundings)]
#![feature(option_take_if)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![allow(dead_code)]

mod bvec;
mod tst;
mod fast_bvec;
mod benchmark;

use tst::Query;

use crate::benchmark::*;
use crate::tst::ExecQueries;
use crate::bvec::RankSelectVector;
use std::io::Write;
use std::io::BufRead;

fn praktikum_main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        panic!("Usage: {} <input> <output>", args[0]);
    }

    let input = &args[1];
    let output = &args[2];

    let bv;
    let mut qs;

    {
        let input = std::fs::File::open(input).unwrap();
        let mut file = std::io::BufReader::new(input);

        let mut n_str = String::new();
        file.read_line(&mut n_str).ok();
        let n = n_str.trim().parse::<usize>().unwrap();

        bv = bvec::BitVector::new_from_input(&mut file);
        qs = Vec::with_capacity(n);
        file.lines().map(|x| x.unwrap()).for_each(|line| {
            let mut line = line.trim().split(' ');
            let cmd = line.next().unwrap();
            let v = line.next().unwrap().parse::<usize>().unwrap();

            qs.push(match cmd {
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
            });
        });
    }

    let used_space;
    let mut answers: Vec<usize> = vec![0; qs.len()];
    let accel_bv;

    let time_build = measure_time!({
        accel_bv = fast_bvec::FastRASBVec::<Params<4096, 32768, 32, 48>>::new(bv);
    });

    let time_query = measure_time!({
        used_space = accel_bv.get_memory_usage();
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

fn main() {
    praktikum_main();
    //benchmark_select_all(&[AllBench::Random, AllBench::RankGeneral, AllBench::SelectGeneral, AllBench::SelectBruteforce]);
    //benchmark_select_all(&[AllBench::RankGeneral]);
}

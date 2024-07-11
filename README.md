# Bit Vector with support for Rank and Select

This repository contains a Rust implementation of a static bit vector data structure with support for O(1) rank and O(logN) select queries.
It has been implemented as an exercise part of the Advanced Data Structures course at the Karlsruhe Institute of Technology.

# Building & running

This library relies on a few unstable rust features, therefore it is highly recommended to use it with latest nightly compiler.
It has been tested to build and work with rust `1.81.0-nightly`.

To run this, first select the nightly compiler:

```
rustup default nightly
rustup update nightly
```

Then, to run the main program use the following command:

```
cargo run --release <path_to_input_file> <path_to_output_file>
```

The input file is expected to have the following format:

```
<N: number of access, rank, select queries>
<bitvector, string of 0s and 1s>
```

Followed by N queries, which are either `access P`, `rank 0|1 P` or `select 0|1 P`.
See `test_data/sample.in` and `test_data/sample.out` for an example.

## Dependencies

The library uses a number of dependencies, which are mostly used for the unit tests and benchmarks provided alongside the bit vector implementation:

- [`num`](https://docs.rs/num/latest/num/) is a crate providing nice integer traits and functions like `div_ceil`.
- [`rand`](https://docs.rs/rand/latest/rand/) and [`rand_xoshiro`](https://docs.rs/rand_xoshiro/latest/rand_xoshiro/) are used for random number generation in many of the tests and benchmarks.
- [`seq-macro`](https://docs.rs/seq-macro/latest/seq_macro/) is used for looping over generic parameters for benchmarks.
- [`prettytable-rs`](https://docs.rs/prettytable-rs/latest/prettytable_rs/) is used for printing tables in the benchmark functions.
- [`colored`](https://docs.rs/colored/latest/colored/) is used for colored output in the benchmark functions.
- [`cfg-if`](https://docs.rs/cfg-if/latest/cfg_if/) is used for conditional compilation with `pdep` instruction support on x86 targets.
- [`derivative`](https://docs.rs/derivative/latest/derivative/) is used for automatic derivation of traits in some structures where the standard Rust `#[derive]` is not sufficient.

## Control flow explanation

For following the control flow of this project, see the `ARCHITECTURE.md` file.

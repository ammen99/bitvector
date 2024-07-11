# General overview

The project is split into multiple files, explanation for each of them follows.

### `src/bvec.rs`

This file contains an implementation of a simple static `BitVector` where bits are packed in 64-bit integers.

`BitVector` also supports the important operations:

- `count_ones(l, r)` which counts the number of set bits in the interval `[l, r)`. It iterates over all `BitCell`s (64-bit words) which at least partially overlap this interval, and sums the ones in it.
- `find_nth_x(start, nth, x)` which finds the `nth` bit which is equal to `x` starting from position `start`. It is implemented in a loop which iterates over BitCells starting from the bitcell containing `s`. When it finds the bit cell which must contain the desired bit, it uses `pdep` to find the appropriate position.

It also defines the trait RankAccessVector which defines all operations required for the project (rank, select0, select1, access).

### `src/fast_bvec.rs`

This is the main part of the project. The general strategy is:

- We split the vector into blocks, superblocks (containing multiple blocks) and megablocks (containing multiple superblocks).
  For megablocks and superblocks, we store the number of set bits before them in the bit vector.
  For plain blocks, we store the number of set bits before the block in the superblock.
  Because we want to support testing with various block, superblock and megablock sizes, most `struct`s in this file have a generic `Parameters` of type `RASBVecParameters`
  which carries information about the exact sizes.

- Blocks and superblocks are stored together in a `RankSuperblock` data structure, which interleaves superblocks and blocks.
  With them, we answer rank queries in O(1).

- Megablocks are stored separately for faster select queries: first, we binary search over megablocks to find the megablock containing the desired bit.
  Then, we iterate over the superblocks in that megablock to find the superblock where the desired bit is.
  Then, we iterate over the blocks in the superblocks to find the block where the desired bit is.
  Finally, we iterate over the bits to find the desired bit.

### `src/tst.rs`

This file contains many different utilities related to generating random queries, the `Query` enum (which contains all possible query types) and
provides `exec_queries` on iterators.

### `src/benchmark.rs`

This file contains the code used for benchmarking the implementation in various configurations (different block size, superblocks, etc.)

### `src/main.rs`

This file contains the main function, as well as the code to parse the input file required for the project.

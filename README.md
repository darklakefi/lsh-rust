# LSH rust tester

Mirroring circuit LSH for faster result analysis.


## Differences from circuit

- Current circom circuit is using different u64 -> i64 conversion. It is simply dropping the MSB. While here it's dropping MSB and using the remainder to calculate the magnitude value (subtract from 63 bit value).


## Scripts

Multiple scripts, no guarantee that all run as older we're abandoned in favor of newer versions.

To generate 64 projection hashes and save comparisons
`cargo run --bin 64_bit`

To generate 128 projection hashes and save comparisons
`cargo run --bin 128_bit`

... Look up cargo.toml for script names and run as above

Latest one

Splits u64 and applies weights to u64 pieces.

`cargo run --bin v3_splitting_weighted`
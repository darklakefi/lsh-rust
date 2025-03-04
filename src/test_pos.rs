use num_bigint::BigInt;
use rand::distr::weighted;
use solana_poseidon::{hashv, Endianness, Parameters};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;


fn generate_lsh_rust() {
    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);

    // projection count (generally recommended 512/1024 for higher precision but lower performance)
    let projection_counter = 0;
    while true {
        let projection_index_bytes = u64::to_le_bytes(projection_counter);

        let mut input_piece_index = 0;

        for j in 0..10 {
            let input_index_bytes = u64::to_le_bytes(input_piece_index);
            input_piece_index += 1;

            let dim0: &[&[u8]] = &[&salt_bytes, &projection_index_bytes, &input_index_bytes];

            let pos_hash0 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
            let pos_hash_bytes0 = pos_hash0.to_bytes();

            let mut array0 = [0u8; 8];
            array0.copy_from_slice(&pos_hash_bytes0[..8]);

            let projection = i64::from_le_bytes(array0);
            if projection > 0 && projection < (1000000000) as i64 {
                println!(
                    "Less than 10^9: {}",
                    projection
                );
                std::process::exit(0);
            }
            // println!("{},", projection);

            // let norm_projection = projection as f64 / 9223372036854775807.0; // 2^63 - 1

            // println!("{},", norm_projection);
        }
    }
}


fn main() {
    generate_lsh_rust();
}

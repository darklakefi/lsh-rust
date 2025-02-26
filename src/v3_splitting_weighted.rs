use num_bigint::BigInt;
use solana_poseidon::{hashv, Endianness, Parameters};
use std::fs::OpenOptions;
use std::io::Write;

fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn hamming_distance_128(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

fn hamming_distance_string(a: &str, b: &str) -> u32 {
    a.chars()
        .zip(b.chars())
        .filter(|(char_a, char_b)| char_a != char_b)
        .count() as u32
}

fn normalize_vector(v: &Vec<f64>) -> Vec<f64> {
    let norm: f64 = v.iter().map(|&x| x * x).sum::<f64>().sqrt();

    if norm == 0.0 {
        v.clone() // Avoid division by zero
    } else {
        v.iter().map(|&x| x / norm).collect()
    }
}

fn split_u64_le(value: u64) -> [u8; 8] {
    value.to_le_bytes()
}

fn split_u64_gradual(value: u64) -> [u64; 6] {
    let mut remaining = value;

    // Define the bit sizes for each segment
    let bit_sizes = [1, 2, 4, 8, 16, 33];
    let mut pieces = [0u64; 6];

    // Start from MSB to LSB
    let mut shift = 64;

    for (i, &size) in bit_sizes.iter().enumerate() {
        shift -= size; // Shift the bit window
        pieces[i] = (remaining >> shift) & ((1 << size) - 1); // Extract the segment
    }

    pieces
}

fn split_u64_into_64(value: u64) -> [u64; 64] {
    let mut results = [0u64; 64]; // Array to store results

    for i in 0..64 {
        let shift = 64 - (i + 1); // Compute shift to extract MSB-first bits
        if i == 63 {
            results[i] = value; // Last iteration takes the full value
        } else {
            results[i] = (value >> shift) & ((1u64 << (i + 1)) - 1);
        }
    }

    results
}

fn split_u64_with_max_bits(value: u64, max_bits: u32) -> Vec<u64> {
    // increasing
    assert!(max_bits <= 64, "max_bits must be between 0 and 64");
    // println!("value: {} | max_bits: {}", value, max_bits);

    // let total_bits = 64 - max_bits; // Number of bits to consider
    let mut results = Vec::with_capacity(max_bits as usize); // Store results dynamically

    for i in 0..max_bits {
        let shift = max_bits - (i + 1); // Compute shift from MSB
        let mask = if i == max_bits - 1 {
            ((1u64 << (max_bits + 1)) - 1)
        } else {
            (1u64 << (i + 1)) - 1 // Create bit mask for current segment
        };
        // println!("shift: {} | mask: {:b}", shift, mask);
        results.push((value >> shift) & mask);
    }

    results
}

fn split_u64_into_weighted_nibbles(value: u64, max_bits: u32) -> Vec<u64> {
    // println!("value: {} | max_bits: {}", value, max_bits);

    assert!(max_bits <= 64, "max_bits must be between 0 and 64");

    let num_chunks = (max_bits + 3) / 4; // Number of 4-bit segments
    let max_bits_rounded = num_chunks * 4;
    let mut results = Vec::with_capacity(num_chunks as usize);

    // println!("num_chunks: {}", num_chunks);

    for i in 0..num_chunks {
        let shift = max_bits_rounded - (i + 1) * 4; // Compute bit shift from MSB

        let nibble = if shift >= 0 {
            (value >> shift) & 0xF // Extract 4-bit segment
        } else {
            0 // If out of range, use 0
        };

        let weight = 1 << (num_chunks - i); // Weight = 2^i
        results.push(nibble * weight);
    }

    results
}

fn bits_needed(n: u64) -> u32 {
    if n == 0 {
        1 // At least 1 bit is needed to store 0
    } else {
        64 - n.leading_zeros() // Number of bits required
    }
}

fn generate_lsh_rust(inputs: &[u64; 4]) -> String {
    let mut hash_res: String = "".to_string();

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);

    let lsh_inputs = inputs;

    for i in 0..128 {
        let index_bytes = u64::to_le_bytes(i);

        let mut input_index = 0;
        let mut final_sum: BigInt = BigInt::from(0);
        let max_bits = 64;

        for &input in lsh_inputs.iter() {
            let mut input_parts = split_u64_into_weighted_nibbles(input, max_bits);
            input_parts.reverse();

            for &input_u8 in input_parts.iter() {
                let input_index_bytes = u64::to_le_bytes(input_index);
                input_index += 1;

                let dim0: &[&[u8]] = &[&salt_bytes, &index_bytes, &input_index_bytes];

                let pos_hash0 =
                    hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
                let pos_hash_bytes0 = pos_hash0.to_bytes();

                let mut array0 = [0u8; 8];
                array0.copy_from_slice(&pos_hash_bytes0[..8]);

                let projection = i64::from_le_bytes(array0);

                let mult0 = input_u8 as i128 * projection as i128;
                // println!(" projection: {} | input: {} | mult0: {} |", projection, input, mult0);
                final_sum += mult0;
                // println!("final_sum: {}", final_sum);
            }
        }

        if final_sum.lt(&BigInt::from(0)) {
            hash_res.push('1');
        } else {
            hash_res.push('0');
        }
    }

    hash_res
}

fn get_hash(is_swap_x_to_y: bool, balance_x: u64, balance_y: u64, input_amount: u64) -> String {
    let k = balance_x as u128 * balance_y as u128;

    let new_balance_x;
    let new_balance_y;
    let output;
    if (is_swap_x_to_y) {
        new_balance_x = balance_x + input_amount;
        new_balance_y = (k / new_balance_x as u128) as u64;
        output = balance_y - new_balance_y;
    } else {
        new_balance_y = balance_y + input_amount;
        new_balance_x = (k / new_balance_y as u128) as u64;
        output = balance_x - new_balance_x;
    }

    // salt is 0 for now
    let input_vector: [u64; 4] = [balance_x, balance_y, new_balance_x, new_balance_y]; // Example input

    let lsh_hash = generate_lsh_rust(&input_vector);
    lsh_hash
}

fn fake_trade_to_y(balance_x: u64, balance_y: u64, input_amount: u64) -> (u64, u64) {
    let k = balance_x as u128 * balance_y as u128;

    let new_balance_x = balance_x + input_amount;
    let new_balance_y = (k / new_balance_x as u128) as u64;

    (new_balance_x, new_balance_y)
}

fn fake_trade_to_x(balance_x: u64, balance_y: u64, input_amount: u64) -> (u64, u64) {
    let k = balance_x as u128 * balance_y as u128;

    let new_balance_y = balance_y + input_amount;
    let new_balance_x = (k / new_balance_y as u128) as u64;

    (new_balance_x, new_balance_y)
}

fn main() {
    let input_amount = 10000000;
    let mut balance_x = 10000000000000;
    let mut balance_y = 80000000000000;

    // user trading to y direction
    let is_swap_x_to_y = true;
    let mut front_run_base = 100;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("v3-split-weight-max-low-low-1-8.csv"))
        .unwrap();

    for i in 0..10 {
        let base_hash = get_hash(is_swap_x_to_y, balance_x, balance_y, input_amount);

        let mut better_balance_x = balance_x;
        let mut better_balance_y = balance_y;
        let mut worse_balance_x = balance_x;
        let mut worse_balance_y = balance_y;
        println!("---Balance RESET---");
        let mut front_run_input = front_run_base;

        writeln!(file, "{}-{}", front_run_input, front_run_input * 9).unwrap();
        writeln!(file, "better,worse").unwrap();

        let mut csv: String = "".to_string();

        for t in 0..9 {
            println!("front_run_input: {}", front_run_input);

            (better_balance_x, better_balance_y) =
                fake_trade_to_x(balance_x, balance_y, front_run_input);
            (worse_balance_x, worse_balance_y) =
                fake_trade_to_y(balance_x, balance_y, front_run_input);

            let better_hash = get_hash(
                is_swap_x_to_y,
                better_balance_x,
                better_balance_y,
                input_amount,
            );
            let worse_hash = get_hash(
                is_swap_x_to_y,
                worse_balance_x,
                worse_balance_y,
                input_amount,
            );

            let better_distance = hamming_distance_string(&base_hash, &better_hash);
            let worse_distance = hamming_distance_string(&base_hash, &worse_hash);

            csv.push_str(&format!("{},{}\n", better_distance, worse_distance));
            front_run_input += front_run_base;
        }

        writeln!(file, "{}", csv).unwrap();

        front_run_base = 100 * 10u64.pow(i + 1);
    }
}

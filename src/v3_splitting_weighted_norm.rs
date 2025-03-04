use num_bigint::BigInt;
use solana_poseidon::{hashv, Endianness, Parameters};
use std::fs::OpenOptions;
use std::io::Write;

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

fn split_u64_into_weighted_nibbles(value: u64, max_bits: u32) -> Vec<u64> {
    assert!(max_bits <= 64, "max_bits must be between 0 and 64");

    let num_chunks = (max_bits + 3) / 4; // Number of 4-bit segments
    let max_bits_rounded = num_chunks * 4;
    let mut results = Vec::with_capacity(num_chunks as usize);

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

fn generate_lsh_rust(inputs: &[u64; 4]) -> String {
    let mut hash_res: String = "".to_string();

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);

    let lsh_inputs = inputs;

    for i in 0..128 {
        let index_bytes = u64::to_le_bytes(i);

        let mut input_index = 0;
        let mut final_sum: BigInt = BigInt::from(0);
        let mut dec_accumulator: f64 = 0.0;

        // use either max bits or uncomment below to limit to the min bits needed
        let max_bits = 64;

        // let max_bits = lsh_inputs.iter().map(|&input| bits_needed(input)).max().unwrap();

        for &input in lsh_inputs.iter() {
            let input_parts = split_u64_into_weighted_nibbles(input, max_bits);
            let input_parts = normalize_vector(&input_parts.iter().map(|&x| x as f64).collect());

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
                let projection = projection as f64 / i64::MAX as f64;

                let mult0 = input_u8 * projection;

                dec_accumulator += mult0.fract();

                if dec_accumulator >= 1.0 {
                    final_sum += 1;
                    dec_accumulator -= 1.0;
                } else if dec_accumulator <= -1.0 {
                    final_sum -= 1;
                    dec_accumulator += 1.0;
                }

                final_sum += mult0.trunc() as i64;
            }
        }

        // for now we ignore the fraction as it is a very small chance it matters

        if (final_sum.eq(&BigInt::from(0))) {
            if dec_accumulator < 0.0 {
                hash_res.push('1');
            } else {
                hash_res.push('0');
            }
        } else {
            if final_sum.lt(&BigInt::from(0)) {
                hash_res.push('1');
            } else {
                hash_res.push('0');
            }
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
    let input_amount = 100;
    let mut balance_x = 10000000000000000;
    let mut balance_y = 20000000000000000;

    // user trading to y direction
    let is_swap_x_to_y = true;
    let mut front_run_base = 100;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("v3-split-weight-norm-max-low-high.csv"))
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

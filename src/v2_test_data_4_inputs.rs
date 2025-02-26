use itertools::izip;
use solana_poseidon::{hashv, Endianness, Parameters};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};

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

fn generate_lsh_rust(inputs: &[u64; 4]) -> String {
    let mut hash_res: String = "".to_string();

    let salt = 0;

    let salt_bytes = u64::to_le_bytes(salt);

    for i in 0..512 {
        let index_bytes = u64::to_le_bytes(i);

        let mut input_index = 0;
        let mut final_sum = 0;
        for &input in inputs.iter() {
            let input_index_bytes = u64::to_le_bytes(input_index);
            input_index += 1;

            let dim0: &[&[u8]] = &[&salt_bytes, &index_bytes, &input_index_bytes];

            let pos_hash0 = hashv(Parameters::Bn254X5, Endianness::LittleEndian, &dim0).unwrap();
            let pos_hash_bytes0 = pos_hash0.to_bytes();

            let mut array0 = [0u8; 8];
            array0.copy_from_slice(&pos_hash_bytes0[..8]);

            let projection = i64::from_le_bytes(array0);

            let mult0 = input as i128 * projection as i128;
            final_sum += mult0;
        }

        if final_sum < 0 {
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

fn binary_search_first_hamming_diff(
    base_hash: &str,
    base_reserve_x: u64,
    base_reserve_y: u64,
    input_amount: u64,
    is_swap_x_to_y: bool, // currently only workse with x to y (true)
    is_in_favor: bool,
) -> (u64, u64, u64) {
    println!(
        "binary search front run: {}",
        if is_in_favor { "in favor" } else { "against" }
    );

    let mut input_low = 0;
    let mut input_high = if is_in_favor {
        base_reserve_y * 2
    } else {
        base_reserve_x * 2
    };
    let mut prev_better_reserve_x = 0;
    let mut prev_better_reserve_y = 0;
    let mut front_run_input = 1;

    let mut first_hamm_reserve_x = 0;
    let mut first_hamm_reserve_y = 0;

    while first_hamm_reserve_x == 0 || first_hamm_reserve_y == 0 {
        println!("Trying front-run with amount: {}", front_run_input);

        let post_front_run_reserve_x;
        let post_front_run_reserve_y;

        // in favor front-runs against the base trade (makes the rate more favorable for the trader)
        if is_in_favor {
            (post_front_run_reserve_x, post_front_run_reserve_y) =
                fake_trade_to_x(base_reserve_x, base_reserve_y, front_run_input);
        } else {
            (post_front_run_reserve_x, post_front_run_reserve_y) =
                fake_trade_to_y(base_reserve_x, base_reserve_y, front_run_input);
        }

        let new_hash = get_hash(
            is_swap_x_to_y,
            post_front_run_reserve_x,
            post_front_run_reserve_y,
            input_amount,
        );

        let hamming_distance = hamming_distance_string(&base_hash, &new_hash);
        println!("hamming_distance: {}", hamming_distance);

        if hamming_distance != 0 {
            let new_input = (front_run_input + input_low) / 2;
            if new_input == front_run_input || new_input == input_low {
                // both values are tested
                first_hamm_reserve_x = prev_better_reserve_x;
                first_hamm_reserve_y = prev_better_reserve_y;
                break;
            }

            input_high = front_run_input;
            front_run_input = new_input;
        } else {
            let new_input = (front_run_input + input_high) / 2;
            if new_input == input_high || new_input == front_run_input {
                // both values are tested
                first_hamm_reserve_x = prev_better_reserve_x;
                first_hamm_reserve_y = prev_better_reserve_y;
                break;
            }

            input_low = front_run_input;
            front_run_input = new_input;
        }

        prev_better_reserve_x = post_front_run_reserve_x;
        prev_better_reserve_y = post_front_run_reserve_y;
    }

    println!("found: {}, {}", first_hamm_reserve_x, first_hamm_reserve_y);

    return (first_hamm_reserve_x, first_hamm_reserve_y, front_run_input);
}

fn main() {
    let file = File::open("raydium.csv").expect("Unable to open file");
    let reader = BufReader::new(file);

    let mut rad_amount_in = Vec::new();
    let mut rad_balance_x = Vec::new();
    let mut rad_balance_y = Vec::new();

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("hamming-res.csv")
        .unwrap();

    writeln!(file, "amount_in,reserve_in,reserve_out,in_favor_amount,first_reserve_in_hamming_in_favor,first_reserve_out_hamming_in_favor,against_amount,first_reserve_in_hamming_against,first_reserve_out_hamming_against").unwrap();

    for (index, line) in reader.lines().enumerate() {
        if (index == 0) {
            // skip headers
            continue;
        }

        let line = line.expect("Unable to read line");
        let values: Vec<&str> = line.split(',').collect();

        if let (
            Some(dec_from),
            Some(dec_to),
            Some(is_reserve_swapped),
            Some(amount_in),
            Some(amount_out),
            Some(reserve_in),
            Some(reserve_out),
        ) = (
            values.get(0),
            values.get(1),
            values.get(2),
            values.get(3),
            values.get(4),
            values.get(5),
            values.get(6),
        ) {
            let is_reserve_swapped: bool = is_reserve_swapped
                .parse()
                .expect("Unable to parse reserve_in");
            let dec_from: f64 = dec_from.parse().expect("Unable to parse reserve_in");
            let dec_to: f64 = dec_to.parse().expect("Unable to parse reserve_in");

            let amount_in: f64 = amount_in.parse().expect("Unable to parse reserve_in");
            let amount_out: f64 = amount_out.parse().expect("Unable to parse reserve_in");
            let reserve_in: f64 = reserve_in.parse().expect("Unable to parse reserve_in");
            let reserve_out: f64 = reserve_out.parse().expect("Unable to parse reserve_out");

            rad_amount_in.push((amount_in * 10f64.powf(dec_from)) as u64);
            rad_balance_x.push(if is_reserve_swapped {
                reserve_out
            } else {
                reserve_in
            } as u64);
            rad_balance_y.push(if is_reserve_swapped {
                reserve_in
            } else {
                reserve_out
            } as u64);
        }
    }

    for (input_amount, balance_x, balance_y) in izip!(
        rad_amount_in.iter(),
        rad_balance_x.iter(),
        rad_balance_y.iter()
    ) {
        println!("input_amount: {}", input_amount);
        println!("balance_x: {}", balance_x);
        println!("balance_y: {}", balance_y);

        // user trading to y direction
        let is_swap_x_to_y = true;

        let base_hash = get_hash(is_swap_x_to_y, *balance_x, *balance_y, *input_amount);

        let mut csv: String = "".to_string();

        let (reserve_x_in_favor, reserve_y_in_favor, in_favor_amount) =
            binary_search_first_hamming_diff(
                &base_hash,
                *balance_x,
                *balance_y,
                *input_amount,
                is_swap_x_to_y,
                true,
            );

        let (reserve_x_against, reserve_y_against, against_amount) =
            binary_search_first_hamming_diff(
                &base_hash,
                *balance_x,
                *balance_y,
                *input_amount,
                is_swap_x_to_y,
                false,
            );

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{}",
            input_amount,
            balance_x,
            balance_y,
            in_favor_amount,
            reserve_x_in_favor,
            reserve_y_in_favor,
            against_amount,
            reserve_x_against,
            reserve_y_against
        ));
        writeln!(file, "{}", csv).unwrap();
    }
}
